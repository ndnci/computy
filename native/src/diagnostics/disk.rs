use crate::domain::{Check, Status, new_id};
use serde_json::json;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

struct ScratchFile(PathBuf);
impl ScratchFile {
    fn create(path: PathBuf) -> std::io::Result<(Self, std::fs::File)> {
        // Acquire ownership only after atomic creation succeeds. A name collision
        // must never give the cleanup guard ownership of someone else's file.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok((Self(path), file))
    }
}
impl Drop for ScratchFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn verify_chunk(actual: &[u8], expected: &[u8]) -> u64 {
    actual.iter().zip(expected).filter(|(a, b)| a != b).count() as u64
        + actual.len().abs_diff(expected.len()) as u64
}
fn pattern(data: &mut [u8], block: usize) {
    for (index, chunk) in data.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        let mut value =
            (index as u64).wrapping_add((block as u64).wrapping_mul(0x9e3779b97f4a7c15));
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        chunk.copy_from_slice(&(value ^ (value >> 31)).to_le_bytes());
    }
}
pub fn run(root: &Path, mib: usize, cancel: &AtomicBool, progress: &dyn Fn(f64, &str)) -> Check {
    let started = Instant::now();
    let mut check = Check::new("disk-roundtrip", Status::Unavailable, "");
    check.component = "storage".into();
    check.name = "Intégrité du fichier temporaire".into();
    check.limitations = vec!["Vérifie uniquement un fichier sur le volume des données de Computy, pas tous les secteurs ni tous les disques.".into(),
        "Les caches de l’OS et du contrôleur peuvent influencer les mesures. Ce contrôle n’est pas un benchmark matériel.".into()];
    if cancel.load(Ordering::Relaxed) {
        check.status = Status::Cancelled;
        check.message = "Test interrompu.".into();
        return check;
    }
    if let Err(error) = std::fs::create_dir_all(root) {
        check.message = format!("Dossier de test inaccessible : {error}");
        return check;
    }
    let (scratch, mut file) =
        match ScratchFile::create(root.join(format!("computy-{}.scratch", new_id()))) {
            Ok(created) => created,
            Err(error) => {
                check.message = format!("Test non conclu : {error}");
                check.duration_ms = started.elapsed().as_millis() as u64;
                return check;
            }
        };
    let mut verified_bytes = 0u64;
    let mut mismatches = 0u64;
    let mut write_ms = 0u128;
    let mut read_ms = 0u128;
    #[cfg(target_os = "linux")]
    let mut cache_advice = false;
    #[cfg(not(target_os = "linux"))]
    let cache_advice = false;
    let result = (|| -> Result<bool, String> {
        let mut expected = vec![0u8; 1024 * 1024];
        let write_start = Instant::now();
        for block in 0..mib {
            if cancel.load(Ordering::Relaxed) {
                return Ok(false);
            }
            pattern(&mut expected, block);
            file.write_all(&expected).map_err(|e| e.to_string())?;
            progress((block + 1) as f64 / (2 * mib) as f64, "Écriture des motifs");
        }
        file.sync_all().map_err(|e| e.to_string())?;
        write_ms = write_start.elapsed().as_millis();
        drop(file);
        let mut file = std::fs::File::open(&scratch.0).map_err(|e| e.to_string())?;
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            // Advisory cache eviction only; no global cache flush or raw device access.
            cache_advice = unsafe {
                libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) == 0
            };
        }
        let read_start = Instant::now();
        let mut actual = vec![0u8; expected.len()];
        for block in 0..mib {
            if cancel.load(Ordering::Relaxed) {
                return Ok(false);
            }
            file.read_exact(&mut actual).map_err(|e| e.to_string())?;
            pattern(&mut expected, block);
            mismatches += verify_chunk(&actual, &expected);
            verified_bytes += actual.len() as u64;
            progress(
                0.5 + (block + 1) as f64 / (2 * mib) as f64,
                "Relecture et comparaison octet par octet",
            );
        }
        read_ms = read_start.elapsed().as_millis();
        Ok(true)
    })();
    check.status = match &result {
        _ if mismatches > 0 => Status::Failed,
        Ok(true) => Status::Passed,
        Ok(false) => Status::Cancelled,
        Err(_) => Status::Unavailable,
    };
    check.message = match result {
        Ok(true) if mismatches == 0 => {
            "Tous les octets relus correspondent aux motifs écrits.".into()
        }
        Ok(_) if mismatches > 0 => format!("{mismatches} octet(s) divergent(s) détecté(s)."),
        Ok(false) => "Test interrompu ; fichier temporaire supprimé.".into(),
        Err(error) => format!("Test non conclu : {error}"),
        _ => "Test incomplet.".into(),
    };
    if let Err(error) = std::fs::remove_file(&scratch.0)
        && scratch.0.exists()
    {
        if check.status == Status::Passed {
            check.status = Status::Warning;
        }
        check.limitations.push(format!(
            "Fichier temporaire à supprimer manuellement : {} ({error})",
            scratch.0.display()
        ));
    }
    check.duration_ms = started.elapsed().as_millis() as u64;
    check.metrics = json!({"file_size_bytes":mib*1024*1024,"verified_bytes":verified_bytes,"mismatched_bytes":mismatches,
        "write_ms":write_ms,"read_ms":read_ms,"cache_eviction_advised":cache_advice,"directory":root.to_string_lossy()});
    check
}

#[cfg(test)]
mod tests {
    use super::ScratchFile;
    #[test]
    fn refused_creation_never_removes_an_existing_file() {
        let path =
            std::env::temp_dir().join(format!("computy-scratch-{}", crate::domain::new_id()));
        std::fs::write(&path, b"existing data").unwrap();
        assert!(ScratchFile::create(path.clone()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"existing data");
        std::fs::remove_file(path).unwrap();
    }
}
