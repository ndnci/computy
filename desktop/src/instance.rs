//! Single instance: a second launch asks the running process to show its
//! window (unless it is a background login launch) and exits.
use futures::{StreamExt, channel::mpsc};
use gpui::App;
use std::{
    io::{BufRead, BufReader, Write},
    sync::Mutex,
};

#[cfg(unix)]
type Listener = std::os::unix::net::UnixListener;
#[cfg(windows)]
type Listener = std::net::TcpListener;

pub enum Outcome {
    Primary(Listener),
    Forwarded,
}

static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);

#[cfg(unix)]
fn endpoint() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(computy_engine::preferences::default_data_dir);
    let _ = std::fs::create_dir_all(&base);
    base.join("computy-desktop.sock")
}

#[cfg(unix)]
pub fn acquire(background: bool) -> Outcome {
    use std::os::unix::net::{UnixListener, UnixStream};
    let path = endpoint();
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.write_all(if background {
            b"background\n"
        } else {
            b"show\n"
        });
        return Outcome::Forwarded;
    }
    let _ = std::fs::remove_file(&path);
    match UnixListener::bind(&path) {
        Ok(listener) => {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            Outcome::Primary(listener)
        }
        // Another instance won the race.
        Err(_) => Outcome::Forwarded,
    }
}

#[cfg(windows)]
fn port_file() -> std::path::PathBuf {
    computy_engine::preferences::default_data_dir().join("desktop.port")
}

#[cfg(windows)]
pub fn acquire(background: bool) -> Outcome {
    use std::net::{TcpListener, TcpStream};
    if let Some(port) = std::fs::read_to_string(port_file())
        .ok()
        .and_then(|p| p.trim().parse::<u16>().ok())
    {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.write_all(if background {
                b"background\n"
            } else {
                b"show\n"
            });
            return Outcome::Forwarded;
        }
    }
    match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => {
            if let Ok(address) = listener.local_addr() {
                let _ = std::fs::create_dir_all(computy_engine::preferences::default_data_dir());
                let _ = std::fs::write(port_file(), address.port().to_string());
            }
            Outcome::Primary(listener)
        }
        Err(_) => Outcome::Forwarded,
    }
}

pub fn keep(listener: Listener) {
    *LISTENER.lock().unwrap_or_else(|e| e.into_inner()) = Some(listener);
}

/// Accept requests from later launches on a thread; apply them on the UI thread.
pub fn listen(cx: &mut App) {
    let Some(listener) = LISTENER.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return;
    };
    let (sender, mut receiver) = mpsc::unbounded::<String>();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            let _ = BufReader::new(stream).take_line(&mut line);
            if sender.unbounded_send(line.trim().to_owned()).is_err() {
                break;
            }
        }
    });
    cx.spawn(async move |cx| {
        while let Some(message) = receiver.next().await {
            if message == "show" {
                let _ = cx.update(|cx| crate::open_main_window(None, cx));
            }
        }
    })
    .detach();
}

trait TakeLine {
    fn take_line(self, line: &mut String) -> std::io::Result<usize>;
}

impl<R: std::io::Read> TakeLine for BufReader<R> {
    fn take_line(mut self, line: &mut String) -> std::io::Result<usize> {
        use std::io::Read;
        let mut limited = BufReader::new((&mut self).take(64));
        limited.read_line(line)
    }
}
