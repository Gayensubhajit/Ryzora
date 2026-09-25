//! Local HTTP media server for Ryzora.
//!
//! WebKitGTK on Linux does NOT forward HTTP Range headers to custom URI scheme
//! handlers (asset://, stream://, etc.). A plain TCP HTTP/1.1 server receives
//! Range headers normally, allowing HTML5 video to seek and stream large files.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::OnceLock;
use std::thread;

static PORT: OnceLock<u16> = OnceLock::new();

/// Start the media server on a random localhost port.
/// Safe to call multiple times; only the first call starts the server.
pub fn start() -> u16 {
    if let Some(&p) = PORT.get() {
        return p;
    }
    let listener = TcpListener::bind("127.0.0.1:0").expect("ryzora media server: bind failed");
    let port = listener
        .local_addr()
        .expect("ryzora media server: no local addr")
        .port();
    PORT.set(port).ok();

    thread::Builder::new()
        .name("ryzora-media-server".into())
        .spawn(move || {
            for incoming in listener.incoming() {
                if let Ok(stream) = incoming {
                    thread::spawn(|| handle(stream));
                }
            }
        })
        .expect("ryzora media server: spawn failed");

    eprintln!("[ryzora] media server listening on 127.0.0.1:{port}");
    port
}

/// Tauri command — returns the media server port.
#[tauri::command]
pub fn get_media_server_port() -> u16 {
    *PORT.get().expect("media server not started")
}

// ── MIME ──────────────────────────────────────────────────────────────────────

fn mime_for(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

// ── Connection handler ─────────────────────────────────────────────────────────

fn handle(mut stream: std::net::TcpStream) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(10)));
    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut reader = BufReader::new(reader_stream);

    // Read request line
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let request_line = request_line.trim().to_owned();

    // Parse: "GET /media?path=... HTTP/1.1"
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let raw_url = parts[1];

    // Read headers until blank line
    let mut range_header: Option<String> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        let lower = trimmed.to_lowercase();
        if lower.starts_with("range:") {
            range_header = Some(trimmed[6..].trim().to_string());
        }
    }
    drop(reader);

    if method != "GET" && method != "HEAD" {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n");
        return;
    }

    // Extract ?path=... query parameter
    let file_path = raw_url
        .split('?')
        .nth(1)
        .and_then(|q| {
            q.split('&').find_map(|kv| {
                let mut it = kv.splitn(2, '=');
                if it.next()? == "path" {
                    Some(
                        percent_encoding::percent_decode_str(it.next().unwrap_or(""))
                            .decode_utf8_lossy()
                            .into_owned(),
                    )
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    if file_path.is_empty() {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
        return;
    }

    // Open file
    let mut file = match File::open(&file_path) {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            } else {
                let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n");
            }
            return;
        }
    };

    let total_len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => {
            let _ = stream.write_all(
                b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n",
            );
            return;
        }
    };

    let mime = mime_for(&file_path);
    let cors = "Access-Control-Allow-Origin: *\r\n";

    // ── Range request (206) ──────────────────────────────────────────────────
    if let Some(range) = range_header.as_deref().and_then(|r| r.strip_prefix("bytes=")) {
        let parts: Vec<&str> = range.splitn(2, '-').collect();
        let start: u64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let requested_end: u64 = parts
            .get(1)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .unwrap_or(u64::MAX);

        if start >= total_len {
            let hdr = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\n\
                 Content-Range: bytes */{total_len}\r\n\
                 {cors}\
                 Content-Length: 0\r\n\r\n"
            );
            let _ = stream.write_all(hdr.as_bytes());
            return;
        }

        // Serve exactly what the browser requested (no artificial size limit)
        let end = requested_end.min(total_len - 1);
        let nbytes = end + 1 - start;

        let hdr = format!(
            "HTTP/1.1 206 Partial Content\r\n\
             Content-Type: {mime}\r\n\
             Content-Length: {nbytes}\r\n\
             Content-Range: bytes {start}-{end}/{total_len}\r\n\
             Accept-Ranges: bytes\r\n\
             {cors}\
             Connection: close\r\n\r\n"
        );
        let _ = stream.write_all(hdr.as_bytes());

        if method == "HEAD" {
            return;
        }

        if file.seek(SeekFrom::Start(start)).is_ok() {
            let mut writer = BufWriter::with_capacity(256 * 1024, stream);
            let mut taken = (&mut file).take(nbytes);
            let _ = std::io::copy(&mut taken, &mut writer);
            let _ = writer.flush();
        }
        return;
    }

    // ── Full file or HEAD (200) ──────────────────────────────────────────────
    let hdr = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {mime}\r\n\
         Content-Length: {total_len}\r\n\
         Accept-Ranges: bytes\r\n\
         {cors}\
         Connection: close\r\n\r\n"
    );
    let _ = stream.write_all(hdr.as_bytes());

    if method == "HEAD" {
        return;
    }

    let mut writer = BufWriter::with_capacity(256 * 1024, stream);
    let _ = std::io::copy(&mut file, &mut writer);
    let _ = writer.flush();
}
