use crate::domain::{Check, Status};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

pub fn run(seconds: u64, cancel: &AtomicBool, progress: &dyn Fn(f64, &str)) -> Check {
    let started = Instant::now();
    let mut check = Check::new(
        "cpu-known-answers",
        Status::Cancelled,
        "Contrôle interrompu.",
    );
    check.component = "cpu".into();
    check.name = "Calculs vérifiés · SHA-256 & entiers".into();
    check.limitations = vec!["Contrôle de calcul borné : ne couvre pas toutes les instructions, le GPU ou la stabilité thermique prolongée.".into(),
        "Un résultat correct pendant ce test ne certifie pas l’absence de défaut matériel.".into()];
    if cancel.load(Ordering::Relaxed) {
        return check;
    }
    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(4);
    let errors = AtomicU64::new(0);
    let iterations = AtomicU64::new(0);
    let deadline = started + Duration::from_secs(seconds);
    thread::scope(|scope| {
        for _ in 0..workers {
            let errors = &errors;
            let iterations = &iterations;
            scope.spawn(move || {
                // Published SHA-256 known-answer vector: one million ASCII 'a' characters.
                let data = vec![b'a'; 1_000_000];
                let expected = [
                    0xcd, 0xc7, 0x6e, 0x5c, 0x99, 0x14, 0xfb, 0x92, 0x81, 0xa1, 0xc7, 0xe2, 0x84,
                    0xd7, 0x3e, 0x67, 0xf1, 0x80, 0x9a, 0x48, 0xa4, 0x97, 0x20, 0x0e, 0x04, 0x6d,
                    0x39, 0xcc, 0xc7, 0x11, 0x2c, 0xd0,
                ];
                while Instant::now() < deadline && !cancel.load(Ordering::Relaxed) {
                    let actual = Sha256::digest(black_box(&data));
                    if actual[..] != black_box(expected) {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                    let sum: u64 = (0..black_box(10_000u64)).map(black_box).sum();
                    if black_box(sum) != 49_995_000 {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                    iterations.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
        while Instant::now() < deadline && !cancel.load(Ordering::Relaxed) {
            progress(
                (started.elapsed().as_secs_f64() / seconds as f64).min(1.0),
                "Calculs parallèles et comparaison des résultats",
            );
            thread::sleep(Duration::from_millis(80));
        }
    });
    let errors = errors.load(Ordering::Relaxed);
    let iterations = iterations.load(Ordering::Relaxed);
    check.status = if errors > 0 {
        Status::Failed
    } else if cancel.load(Ordering::Relaxed) {
        Status::Cancelled
    } else if iterations == 0 {
        Status::Unavailable
    } else {
        Status::Passed
    };
    check.message = match check.status {
        Status::Passed => "Tous les calculs vérifiés correspondent aux valeurs attendues.".into(),
        Status::Failed => format!("{errors} erreur(s) de calcul détectée(s)."),
        Status::Unavailable => "Aucun calcul terminé ; résultat indisponible.".into(),
        _ => "Contrôle interrompu ; résultat incomplet.".into(),
    };
    check.duration_ms = started.elapsed().as_millis() as u64;
    check.metrics = json!({"workers":workers,"iterations":iterations,"errors":errors,"hashed_bytes":iterations*1_000_000,"algorithm":"SHA-256 KAT million-a + somme entière 0..9999"});
    check
}
