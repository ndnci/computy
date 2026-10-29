//! Bounded subprocesses with concurrently drained output, no shell expansion.
use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
pub fn run(program: &str, args: &[&str], seconds: u64) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let out = child.stdout.take().ok_or("Missing stdout")?;
    let err = child.stderr.take().ok_or("Missing stderr")?;
    let read = |pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.take(8 * 1024 * 1024).read_to_end(&mut bytes);
            bytes
        })
    };
    let stdout = read(Box::new(out));
    let stderr = read(Box::new(err));
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{program}: timeout"));
        }
        thread::sleep(Duration::from_millis(40));
    };
    let out = stdout.join().map_err(|_| "Reader stopped")?;
    let err = stderr.join().map_err(|_| "Reader stopped")?;
    if !status.success() {
        return Err(format!(
            "{program}: {}",
            String::from_utf8_lossy(&err)
                .chars()
                .take(1200)
                .collect::<String>()
        ));
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}
