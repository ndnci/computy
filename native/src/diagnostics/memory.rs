use crate::domain::{Check, Status};
use serde::Serialize;
use serde_json::json;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

#[derive(Debug, Serialize)]
pub struct MemoryError {
    pub offset_bytes: usize,
    pub expected: u64,
    pub actual: u64,
}
#[derive(Debug, Default)]
pub struct Verification {
    pub errors: u64,
    pub first_error: Option<MemoryError>,
}
pub fn verify_words(words: &[u64], expected: impl Fn(usize) -> u64) -> Verification {
    let mut result = Verification::default();
    for index in 0..words.len() {
        // SAFETY: index is bounded by the initialized slice; volatile prevents elision.
        let actual = unsafe { std::ptr::read_volatile(words.as_ptr().add(index)) };
        record(&mut result, index, expected(index), actual);
    }
    result
}

fn record(result: &mut Verification, index: usize, expected: u64, actual: u64) {
    if expected != actual {
        result.errors += 1;
        if result.first_error.is_none() {
            result.first_error = Some(MemoryError {
                offset_bytes: index * 8,
                expected,
                actual,
            });
        }
    }
}

struct ResidentBuffer {
    words: Vec<u64>,
    locked: bool,
}
impl ResidentBuffer {
    fn new(bytes: usize) -> Result<Self, String> {
        let mut words = Vec::new();
        words
            .try_reserve_exact(bytes / 8)
            .map_err(|_| "Allocation mémoire refusée.")?;
        words.resize(bytes / 8, 0);
        #[cfg(unix)]
        // SAFETY: pointer and length describe the live, initialized allocation.
        let locked = unsafe { libc::mlock(words.as_ptr().cast(), bytes) == 0 };
        #[cfg(windows)]
        let locked = unsafe {
            windows_sys::Win32::System::Memory::VirtualLock(words.as_ptr().cast(), bytes) != 0
        };
        #[cfg(not(any(unix, windows)))]
        let locked = false;
        Ok(Self { words, locked })
    }
}
impl Drop for ResidentBuffer {
    fn drop(&mut self) {
        if self.locked {
            #[cfg(unix)]
            // SAFETY: the allocation is still alive; unlock precisely the locked range.
            unsafe {
                libc::munlock(self.words.as_ptr().cast(), self.words.len() * 8);
            }
            #[cfg(windows)]
            unsafe {
                windows_sys::Win32::System::Memory::VirtualUnlock(
                    self.words.as_ptr().cast(),
                    self.words.len() * 8,
                );
            }
        }
    }
}

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub fn run(mib: usize, passes: usize, cancel: &AtomicBool, progress: &dyn Fn(f64, &str)) -> Check {
    let started = Instant::now();
    let mut check = Check::new("memory-patterns", Status::Unavailable, "");
    check.component = "memory".into();
    check.name = "Intégrité mémoire · motifs & inversions".into();
    check.limitations = vec![
        "Test en espace utilisateur : seule l’allocation indiquée est vérifiée, pas toute la RAM physique.".into(),
        "Les caches CPU restent actifs ; ce contrôle ne remplace pas un test autonome au démarrage.".into(),
        "Une erreur peut aussi provenir du contrôleur mémoire, du CPU ou de ses réglages.".into(),
    ];
    let mut buffer = match ResidentBuffer::new(mib * 1024 * 1024) {
        Ok(buffer) => buffer,
        Err(message) => {
            check.message = message;
            return check;
        }
    };
    if !buffer.locked {
        check.limitations.push(
            "Verrouillage physique refusé : certaines pages peuvent passer par le swap.".into(),
        );
    }
    let names = [
        "Adresses virtuelles",
        "Inversions · zéros / uns",
        "Inversions · damier",
        "Motifs pseudo-aléatoires",
        "Bits mobiles",
    ];
    let mut result = Verification::default();
    let mut patterns = 0;
    let mut verified_words = 0u64;
    let base = buffer.words.as_ptr() as u64;
    'passes: for pass in 0..passes {
        for (pattern, name) in names.iter().enumerate() {
            let expected = |i: usize| match pattern {
                0 => base.wrapping_add(i as u64 * 8),
                1 => 0,
                2 => 0xaaaaaaaaaaaaaaaa,
                3 => mix(i as u64 ^ (pass as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15)),
                _ => 1u64.rotate_left(((i + pass) % 64) as u32),
            };
            progress(patterns as f64 / (passes * names.len()) as f64, name);
            for i in 0..buffer.words.len() {
                if i % 8192 == 0 && cancel.load(Ordering::Relaxed) {
                    break 'passes;
                }
                // SAFETY: all accesses stay within a uniquely owned initialized allocation.
                unsafe {
                    std::ptr::write_volatile(buffer.words.as_mut_ptr().add(i), expected(i));
                }
            }
            for i in 0..buffer.words.len() {
                if i % 8192 == 0 && cancel.load(Ordering::Relaxed) {
                    break 'passes;
                }
                unsafe {
                    let ptr = buffer.words.as_mut_ptr().add(i);
                    record(&mut result, i, expected(i), std::ptr::read_volatile(ptr));
                    std::ptr::write_volatile(ptr, !expected(i));
                }
                verified_words += 1;
            }
            for i in (0..buffer.words.len()).rev() {
                if i % 8192 == 0 && cancel.load(Ordering::Relaxed) {
                    break 'passes;
                }
                unsafe {
                    let ptr = buffer.words.as_mut_ptr().add(i);
                    record(&mut result, i, !expected(i), std::ptr::read_volatile(ptr));
                    std::ptr::write_volatile(ptr, expected(i));
                }
                verified_words += 1;
            }
            patterns += 1;
        }
    }
    let complete = patterns == passes * names.len();
    check.status = if result.errors > 0 {
        Status::Failed
    } else if complete {
        Status::Passed
    } else {
        Status::Cancelled
    };
    check.message = if result.errors > 0 {
        format!("{} incohérence(s) détectée(s).", result.errors)
    } else if complete {
        "Aucune incohérence dans la zone testée.".into()
    } else {
        "Test interrompu ; contrôle incomplet.".into()
    };
    check.duration_ms = started.elapsed().as_millis() as u64;
    check.metrics = json!({"allocated_bytes":buffer.words.len()*8,"verified_bytes":verified_words*8,
        "errors":result.errors,"first_error":result.first_error,"patterns_completed":patterns,
        "patterns_planned":passes*names.len(),"passes_completed":patterns/names.len(),"locked":buffer.locked});
    if complete {
        progress(1.0, "Mémoire vérifiée");
    }
    check
}
