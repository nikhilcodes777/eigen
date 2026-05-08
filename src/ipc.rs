use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

/// Get the socket path unique to this user.
pub fn socket_path() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/eigen-{uid}.sock"))
}

/// Send a command string to the running daemon.
/// Returns Ok(()) on success or an error if the daemon is unreachable.
pub fn send_command(cmd: &str) -> std::io::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

/// Start the IPC listener on a background thread.
/// Each received line is forwarded to `callback` on the glib main context.
pub fn start_listener<F>(callback: F)
where
    F: Fn(String) + Send + 'static,
{
    let path = socket_path();

    // Clean up stale socket
    let _ = std::fs::remove_file(&path);

    let listener = UnixListener::bind(&path).expect("Failed to bind IPC socket");

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let reader = BufReader::new(stream);
            for line in reader.lines().map_while(Result::ok) {
                callback(line);
            }
        }
    });
}
