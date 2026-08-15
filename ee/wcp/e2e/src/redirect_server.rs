//! A local stand-in for the authentik sign-in page: serves one HTML document
//! that immediately navigates to the `goauthentik.io://` redirect carrying a
//! token, which is what `cef-host`'s resource-request handler is watching for.
//!
//! It also records the `X-Authentik-Platform-Auth-DTH` header value on every
//! request it serves, so tests can assert `cef-host` really injects it rather
//! than only checking the end-to-end outcome.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// What `cef-host` should end up navigating to. Kept as a whole-URL builder
/// so the token and the query-parameter name stay in one place.
fn redirect_url(token: &str) -> String {
    format!(
        "{}callback?{}={token}",
        wire::REDIRECT_PREFIX,
        wire::TOKEN_QUERY_PARAM
    )
}

fn page(token: Option<&str>) -> String {
    // A plain top-level navigation, the same shape the real flow ends with.
    let script = match token {
        Some(token) => {
            let target = redirect_url(token);
            format!("<script>location.href={target:?};</script>")
        }
        None => String::new(),
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>mock sign-in</title></head>\
         <body>mock sign-in{script}</body></html>"
    )
}

pub struct RedirectServer {
    /// The URL to hand back from the mock `interactive_auth_async`.
    pub url: String,
    auth_headers: Arc<Mutex<Vec<Option<String>>>>,
    shutdown: Arc<AtomicBool>,
    local_port: u16,
}

impl RedirectServer {
    /// Serves a page that immediately navigates to the `goauthentik.io://`
    /// redirect carrying `token`, completing the sign-in.
    pub fn start(token: &str) -> std::io::Result<Self> {
        Self::serve(Some(token))
    }

    /// Serves a page that never redirects, leaving the sign-in window open —
    /// the state a user sits in until they authenticate or back out.
    pub fn start_inert() -> std::io::Result<Self> {
        Self::serve(None)
    }

    fn serve(token: Option<&str>) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let local_port = listener.local_addr()?.port();
        let url = format!("http://127.0.0.1:{local_port}/");

        let auth_headers = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let body = page(token);
        let thread_headers = auth_headers.clone();
        let thread_shutdown = shutdown.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if thread_shutdown.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(stream) = stream else { continue };
                // Chromium opens speculative/preconnect sockets that never
                // send a request, so a failed exchange is not an error.
                let _ = serve_one(stream, &body, &thread_headers);
            }
        });

        Ok(Self {
            url,
            auth_headers,
            shutdown,
            local_port,
        })
    }

    /// The auth-header value seen on each request served, in order; `None`
    /// for a request that carried no such header.
    pub fn observed_auth_headers(&self) -> Vec<Option<String>> {
        self.auth_headers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Drop for RedirectServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Unblock the accept loop so the thread observes the flag and exits.
        let _ = TcpStream::connect(("127.0.0.1", self.local_port));
    }
}

fn serve_one(
    mut stream: TcpStream,
    body: &str,
    auth_headers: &Arc<Mutex<Vec<Option<String>>>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }

    let mut auth_header = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case(wire::AUTH_HEADER_NAME)
        {
            auth_header = Some(value.trim().to_string());
        }
    }
    auth_headers
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(auth_header);

    write!(
        stream,
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n{body}",
        body.len()
    )?;
    stream.flush()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn serves_a_page_that_navigates_to_the_token_redirect() {
        let server = RedirectServer::start("tok-123").unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", server.local_port)).unwrap();
        write!(
            stream,
            "GET / HTTP/1.1\r\nHost: localhost\r\n{}: header-abc\r\n\r\n",
            wire::AUTH_HEADER_NAME
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.contains("goauthentik.io://callback?ak-auth-ia-token=tok-123"));
        assert_eq!(
            server.observed_auth_headers(),
            vec![Some("header-abc".to_string())]
        );
    }
}
