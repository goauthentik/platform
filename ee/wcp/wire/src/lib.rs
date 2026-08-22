//! IPC protocol and shared appearance constants between `credprovider` and
//! `browser-host`, kept in one place so the two processes and the `e2e`
//! harness cannot drift apart.

use std::io::{self, Read, Write};

/// Outcome `credprovider` hands back from the sign-in flow. Never sent over
/// the wire: it is built from a [`HostReport`] plus, for `Redirected`, an
/// `ak-sysd` validation call only `credprovider` can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    Completed { username: String },
    Cancelled,
    Failed { reason: String },
}

/// Sent from the browser host to `credprovider` over the result pipe once the
/// flow reaches an end state the host cannot resolve itself: validating the
/// redirect's token needs `ak-sysd`, which only `credprovider` can reach
/// (`BROWSER_PRIVILEGE.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostReport {
    /// The sign-in redirect fired; `url` is the full callback URL to extract
    /// and validate the token from.
    Redirected { url: String },
    /// The window closed — or the provider asked it to — without reaching the
    /// redirect.
    Cancelled,
}

#[derive(Clone, PartialEq, prost::Message)]
struct HostReportProto {
    #[prost(oneof = "HostOutcome", tags = "1, 2")]
    outcome: Option<HostOutcome>,
}

#[derive(Clone, PartialEq, prost::Oneof)]
enum HostOutcome {
    #[prost(string, tag = "1")]
    Redirected(String),
    #[prost(bool, tag = "2")]
    Cancelled(bool),
}

impl From<&HostReport> for HostReportProto {
    fn from(r: &HostReport) -> Self {
        let outcome = match r {
            HostReport::Redirected { url } => HostOutcome::Redirected(url.clone()),
            HostReport::Cancelled => HostOutcome::Cancelled(true),
        };
        HostReportProto {
            outcome: Some(outcome),
        }
    }
}

impl TryFrom<HostReportProto> for HostReport {
    type Error = WireError;

    fn try_from(p: HostReportProto) -> Result<Self, WireError> {
        match p.outcome {
            Some(HostOutcome::Redirected(url)) => Ok(HostReport::Redirected { url }),
            Some(HostOutcome::Cancelled(_)) => Ok(HostReport::Cancelled),
            None => Err(WireError::MissingOutcome),
        }
    }
}

/// Sent from `credprovider` to the browser host over the control pipe.
///
/// A command channel rather than a bare cancel signal because the host is
/// spawned at tile selection, to pay WebView2's startup cost early, and only
/// submitting produces a URL. `StartSignIn` turns a waiting host into a
/// sign-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCommand {
    /// Load `url`, injecting `header_token` on every request, and show the
    /// window once the page is up.
    StartSignIn { url: String, header_token: String },
    /// Close without completing the flow.
    Cancel,
}

#[derive(Clone, PartialEq, prost::Message)]
struct HostCommandProto {
    #[prost(oneof = "CommandKind", tags = "1, 2")]
    command: Option<CommandKind>,
}

#[derive(Clone, PartialEq, prost::Oneof)]
enum CommandKind {
    #[prost(message, tag = "1")]
    StartSignIn(StartSignInProto),
    #[prost(bool, tag = "2")]
    Cancel(bool),
}

#[derive(Clone, PartialEq, prost::Message)]
struct StartSignInProto {
    #[prost(string, tag = "1")]
    url: String,
    #[prost(string, tag = "2")]
    header_token: String,
}

impl From<&HostCommand> for HostCommandProto {
    fn from(c: &HostCommand) -> Self {
        let command = match c {
            HostCommand::StartSignIn { url, header_token } => {
                CommandKind::StartSignIn(StartSignInProto {
                    url: url.clone(),
                    header_token: header_token.clone(),
                })
            }
            HostCommand::Cancel => CommandKind::Cancel(true),
        };
        HostCommandProto {
            command: Some(command),
        }
    }
}

impl TryFrom<HostCommandProto> for HostCommand {
    type Error = WireError;

    fn try_from(p: HostCommandProto) -> Result<Self, WireError> {
        match p.command {
            Some(CommandKind::StartSignIn(start)) => Ok(HostCommand::StartSignIn {
                url: start.url,
                header_token: start.header_token,
            }),
            Some(CommandKind::Cancel(_)) => Ok(HostCommand::Cancel),
            None => Err(WireError::MissingOutcome),
        }
    }
}

#[derive(Debug)]
pub enum WireError {
    Io(io::Error),
    Decoding(prost::DecodeError),
    FrameTooLarge(u32),
    MissingOutcome,
}

impl From<io::Error> for WireError {
    fn from(e: io::Error) -> Self {
        WireError::Io(e)
    }
}

impl From<prost::DecodeError> for WireError {
    fn from(e: prost::DecodeError) -> Self {
        WireError::Decoding(e)
    }
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Io(e) => write!(f, "pipe I/O error: {e}"),
            WireError::Decoding(e) => write!(f, "frame decoding error: {e}"),
            WireError::FrameTooLarge(n) => write!(f, "frame of {n} bytes exceeds limit"),
            WireError::MissingOutcome => write!(f, "HostReport frame had no outcome set"),
        }
    }
}

impl std::error::Error for WireError {}

/// Every real message here is a short URL or token, so this only guards
/// against a corrupt length prefix becoming an unbounded allocation.
const MAX_FRAME_BYTES: u32 = 64 * 1024;

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(4 + payload.len());
    framed.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    framed.extend_from_slice(payload);
    framed
}

/// Write one length-prefixed protobuf frame to `w` and flush it.
pub fn write_frame<T: prost::Message, W: Write>(w: &mut W, msg: &T) -> Result<(), WireError> {
    w.write_all(&frame(&msg.encode_to_vec()))?;
    w.flush()?;
    Ok(())
}

/// Read exactly one length-prefixed protobuf frame from `r`.
///
/// `Ok(None)` means EOF before any bytes of a new frame arrived — the write end
/// closed cleanly, e.g. the writer exited without sending a result — which
/// callers treat the same as an explicit cancellation.
pub fn read_frame<T: prost::Message + Default, R: Read>(r: &mut R) -> Result<Option<T>, WireError> {
    let mut len_buf = [0u8; 4];
    let mut read = 0usize;
    while read < 4 {
        match r.read(&mut len_buf[read..])? {
            0 if read == 0 => return Ok(None),
            0 => {
                return Err(WireError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "pipe closed mid length-prefix",
                )));
            }
            n => read += n,
        }
    }
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME_BYTES {
        return Err(WireError::FrameTooLarge(len));
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload)?;
    Ok(Some(T::decode(payload.as_slice())?))
}

/// Write a `HostReport` over the result pipe.
pub fn write_host_report<W: Write>(w: &mut W, report: &HostReport) -> Result<(), WireError> {
    write_frame(w, &HostReportProto::from(report))
}

/// Write a `HostCommand` over the control pipe.
pub fn write_host_command<W: Write>(w: &mut W, command: &HostCommand) -> Result<(), WireError> {
    write_frame(w, &HostCommandProto::from(command))
}

/// Read a `HostCommand` from the control pipe. See [`read_frame`] for EOF
/// handling — the host treats a closed control pipe as a cancellation.
pub fn read_host_command<R: Read>(r: &mut R) -> Result<Option<HostCommand>, WireError> {
    match read_frame::<HostCommandProto, R>(r)? {
        Some(proto) => Ok(Some(HostCommand::try_from(proto)?)),
        None => Ok(None),
    }
}

/// Read a `HostReport` from the result pipe. See [`read_frame`] for EOF
/// handling.
pub fn read_host_report<R: Read>(r: &mut R) -> Result<Option<HostReport>, WireError> {
    match read_frame::<HostReportProto, R>(r)? {
        Some(proto) => Ok(Some(HostReport::try_from(proto)?)),
        None => Ok(None),
    }
}

/// Pulls the interactive-auth token out of the sign-in redirect's query
/// string. `None` covers both an unparseable URL and a well-formed one missing
/// the parameter; `credprovider` fails validation either way.
pub fn extract_token(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == TOKEN_QUERY_PARAM)
        .map(|(_, v)| v.into_owned())
}

/// The four credential-provider tile fields, in display order. Field IDs are
/// their index in this slice.
pub const TILE_FIELDS: &[TileField] = &[
    TileField {
        kind: FieldKind::TileImage,
        text: "",
    },
    TileField {
        kind: FieldKind::HiddenLabel,
        text: "authentik",
    },
    TileField {
        kind: FieldKind::LargeText,
        text: "Sign in with authentik",
    },
    TileField {
        kind: FieldKind::SubmitButton,
        text: "Submit",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    TileImage,
    HiddenLabel,
    LargeText,
    SubmitButton,
}

#[derive(Debug, Clone, Copy)]
pub struct TileField {
    pub kind: FieldKind,
    pub text: &'static str,
}

/// Fixed size of the sign-in window.
pub const WINDOW_WIDTH: i32 = 560;
pub const WINDOW_HEIGHT: i32 = 670;

/// Undecorated, so nobody reads this — it is how the sign-in window is told
/// apart from the helper windows WebView2 opens in the same process.
/// `CreateWindowExW` records it either way, so `GetWindowTextW` can read it
/// cross-process without the window having a caption.
pub const WINDOW_TITLE: &str = "Sign in with authentik";

/// `redirect_uri` prefix the sign-in flow completes on.
pub const REDIRECT_PREFIX: &str = "goauthentik.io://";

/// Query parameter on that redirect carrying the token `credprovider`
/// validates against `ak-sysd` to turn a finished sign-in into a username.
pub const TOKEN_QUERY_PARAM: &str = "ak-auth-ia-token";

/// Header carrying the interactive-auth session token, injected on every
/// request the sign-in window makes.
pub const AUTH_HEADER_NAME: &str = "X-Authentik-Platform-Auth-DTH";

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_redirected_through_a_stream() {
        let msg = HostReport::Redirected {
            url: format!("{}callback?state=xyz", REDIRECT_PREFIX),
        };
        let mut buf = Vec::new();
        write_host_report(&mut buf, &msg).unwrap();

        let mut cursor = io::Cursor::new(buf);
        let decoded = read_host_report(&mut cursor).unwrap().unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trips_cancelled() {
        let mut buf = Vec::new();
        write_host_report(&mut buf, &HostReport::Cancelled).unwrap();
        let mut cursor = io::Cursor::new(buf);
        let decoded = read_host_report(&mut cursor).unwrap().unwrap();
        assert_eq!(HostReport::Cancelled, decoded);
    }

    #[test]
    fn read_frame_reports_clean_eof_as_none() {
        let mut cursor = io::Cursor::new(Vec::<u8>::new());
        let decoded = read_host_report(&mut cursor).unwrap();
        assert!(decoded.is_none());
    }

    #[test]
    fn read_frame_rejects_oversized_length_prefix() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(MAX_FRAME_BYTES + 1).to_le_bytes());
        let mut cursor = io::Cursor::new(buf);
        let result = read_host_report(&mut cursor);
        assert!(matches!(result, Err(WireError::FrameTooLarge(_))));
    }

    #[test]
    fn extracts_token_from_a_redirect_url() {
        let url = format!("{REDIRECT_PREFIX}callback?{TOKEN_QUERY_PARAM}=abc123");
        assert_eq!(extract_token(&url), Some("abc123".to_string()));
    }

    #[test]
    fn extracts_token_alongside_other_query_params() {
        let url = format!("{REDIRECT_PREFIX}callback?state=xyz&{TOKEN_QUERY_PARAM}=abc123&code=9");
        assert_eq!(extract_token(&url), Some("abc123".to_string()));
    }

    #[test]
    fn extract_token_is_none_when_the_param_is_absent() {
        let url = format!("{REDIRECT_PREFIX}callback?state=xyz");
        assert_eq!(extract_token(&url), None);
    }

    #[test]
    fn extract_token_is_none_for_an_unparseable_url() {
        assert_eq!(extract_token("not a url"), None);
    }

    /// The sign-in URL and its header token only exist on the `credprovider`
    /// side, so this frame is the only way a preloaded host learns them.
    #[test]
    fn host_commands_round_trip() {
        for command in [
            HostCommand::StartSignIn {
                url: "https://authentik.company/if/flow/default/".to_string(),
                header_token: "header-token".to_string(),
            },
            HostCommand::Cancel,
        ] {
            let mut buf = Vec::new();
            write_host_command(&mut buf, &command).unwrap();
            let mut cursor = io::Cursor::new(buf);
            assert_eq!(read_host_command(&mut cursor).unwrap(), Some(command));
        }
    }

    #[test]
    fn a_closed_control_pipe_reads_as_no_command() {
        let mut cursor = io::Cursor::new(Vec::<u8>::new());
        assert_eq!(read_host_command(&mut cursor).unwrap(), None);
    }

    #[test]
    fn tile_fields_match_current_appearance() {
        assert_eq!(TILE_FIELDS.len(), 4);
        assert_eq!(TILE_FIELDS[2].text, "Sign in with authentik");
        assert_eq!(TILE_FIELDS[3].text, "Submit");
    }
}
