// Localhost HTTP + SSE server behind the live browser preview.
//
// Serves each previewed document at http://127.0.0.1:<port>/<stem>/, serves
// the document's relative assets (images, fonts) from its source directory,
// and pushes a server-sent event on every re-render so the open page replaces
// itself without a manual reload. Hand-rolled over std::net on purpose: a
// single-user localhost GET server does not justify an HTTP dependency.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Path segment (relative to a preview page) of the SSE reload stream.
pub const EVENTS_SEGMENT: &str = "__mjml-preview-events__";

/// A document registered for live preview.
struct PreviewDoc {
    /// Rendered HTML (or error page) for the latest document text.
    html: String,
    /// The source document's URI; refreshes from a same-stem sibling file
    /// must not overwrite this preview.
    uri: String,
    /// Directory of the source document; relative assets are served from here.
    source_dir: Option<PathBuf>,
    /// Grows on every update; SSE subscribers wake when it does.
    version: u64,
}

/// Preview documents shared between the LSP main loop (writer) and the server
/// threads (readers).
#[derive(Default)]
pub struct PreviewState {
    docs: Mutex<HashMap<String, PreviewDoc>>,
    changed: Condvar,
}

impl PreviewState {
    fn lock_docs(&self) -> std::sync::MutexGuard<'_, HashMap<String, PreviewDoc>> {
        self.docs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers (or replaces) the preview for `stem`.
    pub fn register(&self, stem: &str, uri: &str, html: String, source_dir: Option<PathBuf>) {
        let mut docs = self.lock_docs();
        let version = docs.get(stem).map_or(0, |doc| doc.version) + 1;
        docs.insert(
            stem.to_string(),
            PreviewDoc {
                html,
                uri: uri.to_string(),
                source_dir,
                version,
            },
        );
        drop(docs);
        self.changed.notify_all();
    }

    /// Replaces the HTML for `stem` when it is registered to `uri`, waking SSE
    /// subscribers. Returns whether an update happened.
    pub fn refresh(&self, stem: &str, uri: &str, html: String) -> bool {
        let mut docs = self.lock_docs();
        let updated = match docs.get_mut(stem) {
            Some(doc) if doc.uri == uri => {
                doc.html = html;
                doc.version += 1;
                true
            }
            _ => false,
        };
        drop(docs);
        if updated {
            self.changed.notify_all();
        }
        updated
    }

    /// Whether `stem` is registered to `uri` (i.e. refreshes will be served).
    pub fn is_registered(&self, stem: &str, uri: &str) -> bool {
        self.lock_docs().get(stem).is_some_and(|doc| doc.uri == uri)
    }

    /// The latest HTML for `stem`, as the page route serves it.
    pub fn html(&self, stem: &str) -> Option<String> {
        self.lock_docs().get(stem).map(|doc| doc.html.clone())
    }

    fn source_dir(&self, stem: &str) -> Option<PathBuf> {
        self.lock_docs().get(stem).and_then(|doc| doc.source_dir.clone())
    }

    fn version(&self, stem: &str) -> Option<u64> {
        self.lock_docs().get(stem).map(|doc| doc.version)
    }

    /// Blocks until `stem`'s version exceeds `last` (returning the new
    /// version) or `timeout` elapses (returning `None`).
    fn wait_newer_than(&self, stem: &str, last: u64, timeout: Duration) -> Option<u64> {
        let deadline = Instant::now() + timeout;
        let mut docs = self.lock_docs();
        loop {
            if let Some(version) = docs.get(stem).map(|doc| doc.version) {
                if version > last {
                    return Some(version);
                }
            }
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())?;
            docs = self
                .changed
                .wait_timeout(docs, remaining)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// How an incoming request path maps onto the preview server's resources.
#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    /// `/<stem>/` — the rendered preview page.
    Page(String),
    /// `/<stem>` — redirect to `/<stem>/` so relative asset URLs resolve.
    RedirectToPage(String),
    /// `/<stem>/__mjml-preview-events__` — the SSE reload stream.
    Events(String),
    /// `/<stem>/<relative path>` — an asset next to the source document.
    Asset(String, String),
    /// Anything else.
    NotFound,
}

/// Maps a raw request path (still percent-encoded, possibly with a query
/// string) to a route.
pub fn route(path: &str) -> Route {
    let path = path.split('?').next().unwrap_or(path);
    let path = percent_decode(path);
    let Some(rest) = path.strip_prefix('/') else {
        return Route::NotFound;
    };
    if rest.is_empty() {
        return Route::NotFound;
    }
    match rest.split_once('/') {
        None => Route::RedirectToPage(rest.to_string()),
        Some((stem, "")) => Route::Page(stem.to_string()),
        Some((stem, tail)) if tail == EVENTS_SEGMENT => Route::Events(stem.to_string()),
        Some((stem, tail)) => Route::Asset(stem.to_string(), tail.to_string()),
    }
}

/// Percent-decodes a URL path or path segment. Malformed escapes are passed
/// through unchanged.
pub fn percent_decode(input: &str) -> String {
    fn hex_val(byte: u8) -> Option<u8> {
        char::from(byte).to_digit(16).and_then(|v| u8::try_from(v).ok())
    }

    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let decoded = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| Some(hex_val(bytes[i + 1])? * 16 + hex_val(bytes[i + 2])?))
            .flatten();
        if let Some(byte) = decoded {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encodes a string for use as a single URL path segment.
pub fn percent_encode_segment(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    out
}

/// Filesystem directory of a `file://` document URI (percent-decoded), or
/// `None` for non-file URIs. Assets relative to a preview are served from it.
pub fn file_uri_dir(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let dir_end = path.rfind('/')?;
    let dir = percent_decode(&path[..=dir_end]);
    // Windows file URIs look like file:///C:/dir/; drop the leading slash so
    // the result is a usable drive-letter path.
    let dir = if cfg!(windows) && dir.len() >= 3 && dir.as_bytes()[0] == b'/' && dir.as_bytes()[2] == b':' {
        dir[1..].to_string()
    } else {
        dir
    };
    Some(PathBuf::from(dir))
}

/// Resolves a relative asset path inside `source_dir`, rejecting any path
/// that could escape it (`..`, absolute paths, drive prefixes).
pub fn resolve_asset(source_dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel);
    if rel
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(source_dir.join(rel))
}

/// The Content-Type served for a file, from its extension.
pub fn content_type_for(path: &str) -> &'static str {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map_or_else(String::new, str::to_ascii_lowercase);
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "text/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

/// A minimal SSE frame carrying the new document version.
pub fn sse_event(version: u64) -> String {
    format!("data: {version}\n\n")
}

/// Inserts the live-reload script before `</body>` (or appends it when there
/// is none): subscribes to the SSE endpoint and, on each event, fetches the
/// latest render and swaps the whole document in place, which keeps scroll
/// position and avoids flicker.
pub fn inject_reload_script(html: &str) -> String {
    let script = format!(
        "<script>(function () {{\n\
           var events = new EventSource(\"{EVENTS_SEGMENT}\");\n\
           events.onmessage = function () {{\n\
             fetch(window.location.href, {{ cache: \"no-store\" }})\n\
               .then(function (response) {{ return response.text(); }})\n\
               .then(function (html) {{\n\
                 var doc = new DOMParser().parseFromString(html, \"text/html\");\n\
                 document.documentElement.replaceWith(doc.documentElement);\n\
               }});\n\
           }};\n\
         }})();</script>"
    );
    html.rfind("</body>").map_or_else(
        || format!("{html}{script}"),
        |pos| format!("{}{script}{}", &html[..pos], &html[pos..]),
    )
}

/// Binds the preview server on an ephemeral localhost port and serves
/// requests on background threads. Returns the port.
pub fn start(state: Arc<PreviewState>) -> std::io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let state = Arc::clone(&state);
            std::thread::spawn(move || handle_connection(stream, &state));
        }
    });
    Ok(port)
}

/// Reads one request off the socket and answers it. SSE requests keep the
/// connection (and this thread) until the browser closes it.
fn handle_connection(stream: TcpStream, state: &PreviewState) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(reader_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(reader_stream);
    let Some(path) = read_get_path(&mut reader) else {
        respond_bytes(stream, &response_bytes("400 Bad Request", "text/plain", b"bad request"));
        return;
    };
    match route(&path) {
        Route::Page(stem) => match state.html(&stem) {
            Some(html) => respond_bytes(
                stream,
                &response_bytes(
                    "200 OK",
                    "text/html; charset=utf-8",
                    inject_reload_script(&html).as_bytes(),
                ),
            ),
            None => respond_not_found(stream),
        },
        Route::RedirectToPage(stem) => {
            let location = format!("/{}/", percent_encode_segment(&stem));
            let head = format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            respond_bytes(stream, head.as_bytes());
        }
        Route::Events(stem) => serve_events(stream, state, &stem),
        Route::Asset(stem, rel) => {
            let file = state
                .source_dir(&stem)
                .and_then(|dir| resolve_asset(&dir, &rel))
                .and_then(|path| std::fs::read(path).ok());
            match file {
                Some(body) => respond_bytes(
                    stream,
                    &response_bytes("200 OK", content_type_for(&rel), &body),
                ),
                None => respond_not_found(stream),
            }
        }
        Route::NotFound => respond_not_found(stream),
    }
}

/// Reads the request line and drains the headers, returning the path of a
/// well-formed GET and `None` for anything else.
fn read_get_path(reader: &mut impl BufRead) -> Option<String> {
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?.to_string();
    loop {
        let mut header = String::new();
        let read = reader.read_line(&mut header).ok()?;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
    }
    (method == "GET").then_some(path)
}

/// Streams SSE reload events for `stem` until the browser disconnects. The
/// periodic keep-alive comment doubles as the disconnect probe: writing to a
/// closed socket errors and lets the thread exit.
fn serve_events(mut stream: TcpStream, state: &PreviewState, stem: &str) {
    let mut last = state.version(stem).unwrap_or(0);
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n";
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    loop {
        let payload = state
            .wait_newer_than(stem, last, Duration::from_secs(15))
            .map_or_else(
                || ":keep-alive\n\n".to_string(),
                |version| {
                    last = version;
                    sse_event(version)
                },
            );
        if stream.write_all(payload.as_bytes()).is_err() || stream.flush().is_err() {
            return;
        }
    }
}

fn respond_not_found(stream: TcpStream) {
    respond_bytes(stream, &response_bytes("404 Not Found", "text/plain", b"not found"));
}

fn respond_bytes(mut stream: TcpStream, bytes: &[u8]) {
    let _ = stream.write_all(bytes);
    let _ = stream.flush();
}

/// A complete `Connection: close` HTTP response.
fn response_bytes(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    // --- routing -------------------------------------------------------

    #[test]
    fn route_trailing_slash_is_page() {
        assert_eq!(route("/doc/"), Route::Page("doc".to_string()));
    }

    #[test]
    fn route_bare_stem_redirects_to_page() {
        assert_eq!(route("/doc"), Route::RedirectToPage("doc".to_string()));
    }

    #[test]
    fn route_events_segment_is_events() {
        assert_eq!(
            route(&format!("/doc/{EVENTS_SEGMENT}")),
            Route::Events("doc".to_string())
        );
    }

    #[test]
    fn route_extra_segments_are_assets() {
        assert_eq!(
            route("/doc/img/logo.png"),
            Route::Asset("doc".to_string(), "img/logo.png".to_string())
        );
    }

    #[test]
    fn route_root_is_not_found() {
        assert_eq!(route("/"), Route::NotFound);
        assert_eq!(route(""), Route::NotFound);
    }

    #[test]
    fn route_strips_query_string() {
        assert_eq!(route("/doc/?x=1"), Route::Page("doc".to_string()));
    }

    #[test]
    fn route_percent_decodes_segments() {
        assert_eq!(route("/my%20doc/"), Route::Page("my doc".to_string()));
        assert_eq!(
            route("/doc/img%2Flogo.png"),
            Route::Asset("doc".to_string(), "img/logo.png".to_string())
        );
    }

    // --- percent encoding/decoding --------------------------------------

    #[test]
    fn percent_decode_decodes_escapes() {
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
    }

    #[test]
    fn percent_decode_passes_malformed_escapes_through() {
        assert_eq!(percent_decode("100%zz"), "100%zz");
        assert_eq!(percent_decode("dangling%2"), "dangling%2");
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn percent_encode_segment_keeps_unreserved_and_escapes_rest() {
        assert_eq!(percent_encode_segment("Az09-_.~"), "Az09-_.~");
        assert_eq!(percent_encode_segment("my doc"), "my%20doc");
        assert_eq!(percent_encode_segment("a/b"), "a%2Fb");
    }

    #[test]
    fn percent_encode_then_decode_round_trips() {
        let original = "wild name %/?#[]";
        assert_eq!(percent_decode(&percent_encode_segment(original)), original);
    }

    // --- file URIs and asset resolution ---------------------------------

    #[test]
    #[cfg(not(windows))]
    fn file_uri_dir_strips_filename_and_decodes() {
        assert_eq!(
            file_uri_dir("file:///Users/x/My%20Proj/a.mjml"),
            Some(PathBuf::from("/Users/x/My Proj/"))
        );
    }

    #[test]
    fn file_uri_dir_rejects_non_file_uris() {
        assert_eq!(file_uri_dir("untitled:Untitled-1"), None);
        assert_eq!(file_uri_dir("https://example.com/a.mjml"), None);
    }

    #[test]
    fn resolve_asset_joins_normal_relative_paths() {
        let dir = Path::new("/srv/mail");
        assert_eq!(
            resolve_asset(dir, "img/logo.png"),
            Some(PathBuf::from("/srv/mail/img/logo.png"))
        );
    }

    #[test]
    fn resolve_asset_rejects_traversal_and_absolute_paths() {
        let dir = Path::new("/srv/mail");
        assert_eq!(resolve_asset(dir, "../secret"), None);
        assert_eq!(resolve_asset(dir, "img/../../secret"), None);
        assert_eq!(resolve_asset(dir, "/etc/passwd"), None);
    }

    // --- content types, SSE frames, reload script -----------------------

    #[test]
    fn content_type_for_common_extensions() {
        assert_eq!(content_type_for("a.png"), "image/png");
        assert_eq!(content_type_for("a.JPG"), "image/jpeg");
        assert_eq!(content_type_for("a.svg"), "image/svg+xml");
        assert_eq!(content_type_for("a.woff2"), "font/woff2");
        assert_eq!(content_type_for("unknown.xyz"), "application/octet-stream");
        assert_eq!(content_type_for("noext"), "application/octet-stream");
    }

    #[test]
    fn sse_event_frames_version() {
        assert_eq!(sse_event(7), "data: 7\n\n");
    }

    #[test]
    fn inject_reload_script_lands_before_body_close() {
        let out = inject_reload_script("<html><body>hi</body></html>");
        let script_pos = out.find("<script>").expect("script should be injected");
        let body_close = out.find("</body>").expect("body close should survive");
        assert!(script_pos < body_close, "script should precede </body>: {out}");
        assert!(out.contains("EventSource"), "script should subscribe to SSE: {out}");
        assert!(out.contains(EVENTS_SEGMENT), "script should target the events route: {out}");
    }

    #[test]
    fn inject_reload_script_appends_without_body() {
        let out = inject_reload_script("<p>fragment</p>");
        assert!(out.starts_with("<p>fragment</p>"), "content should be kept: {out}");
        assert!(out.contains("EventSource"), "script should still be added: {out}");
    }

    // --- state -----------------------------------------------------------

    #[test]
    fn refresh_updates_only_matching_uri() {
        let state = PreviewState::default();
        state.register("doc", "file:///a/doc.mjml", "v1".to_string(), None);
        assert!(state.is_registered("doc", "file:///a/doc.mjml"));
        assert!(!state.is_registered("doc", "file:///b/doc.mjml"));
        assert!(!state.refresh("doc", "file:///b/doc.mjml", "evil".to_string()));
        assert!(state.refresh("doc", "file:///a/doc.mjml", "v2".to_string()));
        assert_eq!(state.html("doc"), Some("v2".to_string()));
    }

    #[test]
    fn refresh_bumps_version() {
        let state = PreviewState::default();
        state.register("doc", "u", "v1".to_string(), None);
        let before = state.version("doc").expect("registered doc has a version");
        state.refresh("doc", "u", "v2".to_string());
        let after = state.version("doc").expect("registered doc has a version");
        assert!(after > before, "refresh should bump the version: {before} -> {after}");
    }

    #[test]
    fn wait_newer_than_times_out_without_updates() {
        let state = PreviewState::default();
        state.register("doc", "u", "v1".to_string(), None);
        let last = state.version("doc").unwrap();
        assert_eq!(
            state.wait_newer_than("doc", last, Duration::from_millis(50)),
            None
        );
    }

    #[test]
    fn wait_newer_than_wakes_on_refresh() {
        let state = Arc::new(PreviewState::default());
        state.register("doc", "u", "v1".to_string(), None);
        let last = state.version("doc").unwrap();
        let waiter = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.wait_newer_than("doc", last, Duration::from_secs(5)))
        };
        std::thread::sleep(Duration::from_millis(20));
        state.refresh("doc", "u", "v2".to_string());
        let woken = waiter.join().expect("waiter thread should not panic");
        assert_eq!(woken, Some(last + 1));
    }

    // --- live server -----------------------------------------------------

    /// Sends a GET and returns the whole response (headers + body) as text.
    fn http_get(port: u16, path: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect should succeed");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout should be settable");
        write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").expect("write");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read");
        response
    }

    fn test_server() -> (Arc<PreviewState>, u16) {
        let state = Arc::new(PreviewState::default());
        let port = start(Arc::clone(&state)).expect("server should bind an ephemeral port");
        (state, port)
    }

    #[test]
    fn serves_registered_page_with_reload_script() {
        let (state, port) = test_server();
        state.register("doc", "u", "<html><body>hello</body></html>".to_string(), None);
        let response = http_get(port, "/doc/");
        assert!(response.starts_with("HTTP/1.1 200"), "expected 200: {response}");
        assert!(response.contains("text/html"), "expected html content type: {response}");
        assert!(response.contains("hello"), "expected the rendered body: {response}");
        assert!(response.contains("EventSource"), "expected the reload script: {response}");
    }

    #[test]
    fn serves_404_for_unknown_page() {
        let (_state, port) = test_server();
        let response = http_get(port, "/nope/");
        assert!(response.starts_with("HTTP/1.1 404"), "expected 404: {response}");
    }

    #[test]
    fn redirects_bare_stem_to_trailing_slash() {
        let (state, port) = test_server();
        state.register("doc", "u", "<html></html>".to_string(), None);
        let response = http_get(port, "/doc");
        assert!(response.starts_with("HTTP/1.1 302"), "expected 302: {response}");
        assert!(response.contains("Location: /doc/"), "expected redirect target: {response}");
    }

    #[test]
    fn serves_assets_from_source_dir_and_blocks_traversal() {
        let dir = std::env::temp_dir().join("mjml_preview_server_asset_test");
        std::fs::create_dir_all(&dir).expect("test dir should be creatable");
        std::fs::write(dir.join("logo.png"), b"png-bytes").expect("asset should be writable");

        let (state, port) = test_server();
        state.register("doc", "u", "<html></html>".to_string(), Some(dir.clone()));

        let ok = http_get(port, "/doc/logo.png");
        assert!(ok.starts_with("HTTP/1.1 200"), "expected 200: {ok}");
        assert!(ok.contains("image/png"), "expected png content type: {ok}");
        assert!(ok.contains("png-bytes"), "expected file bytes: {ok}");

        let missing = http_get(port, "/doc/absent.png");
        assert!(missing.starts_with("HTTP/1.1 404"), "expected 404: {missing}");

        let traversal = http_get(port, "/doc/../secret");
        assert!(traversal.starts_with("HTTP/1.1 404"), "expected 404: {traversal}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_non_get_requests() {
        let (_state, port) = test_server();
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout should be settable");
        write!(stream, "POST /doc/ HTTP/1.1\r\nHost: localhost\r\n\r\n").expect("write");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.1 400"), "expected 400: {response}");
    }

    #[test]
    fn sse_stream_delivers_event_on_refresh() {
        let (state, port) = test_server();
        state.register("doc", "u", "v1".to_string(), None);

        let stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout should be settable");
        let mut writer = stream.try_clone().expect("clone");
        write!(writer, "GET /doc/{EVENTS_SEGMENT} HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("write");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        // Read response headers; the server snapshots the version before these
        // are written, so a refresh after this point must produce an event.
        loop {
            line.clear();
            reader.read_line(&mut line).expect("headers should arrive");
            if line == "\r\n" {
                break;
            }
            assert!(!line.is_empty(), "connection closed before headers ended");
        }

        state.refresh("doc", "u", "v2".to_string());

        loop {
            line.clear();
            reader.read_line(&mut line).expect("an event line should arrive");
            if line.starts_with("data: ") {
                break;
            }
        }
        assert_eq!(line, "data: 2\n", "register is version 1, refresh bumps to 2");
    }
}
