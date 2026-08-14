//! IPC protocol and shared appearance constants between `credprovider` and
//! `cef-host`, kept in one place so the two processes and the `e2e` harness
//! can't drift out of sync.

use std::io::{self, Read, Write};

/// Result of the browser sign-in flow, sent from `cef-host` to `credprovider`
/// over the result pipe. The public shape stays a plain enum; wire encoding
/// goes through `AuthResultProto` below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    Completed { username: String },
    Cancelled,
    Failed { reason: String },
}

#[derive(Clone, PartialEq, prost::Message)]
struct AuthResultProto {
    #[prost(oneof = "AuthOutcome", tags = "1, 2, 3")]
    outcome: Option<AuthOutcome>,
}

#[derive(Clone, PartialEq, prost::Oneof)]
enum AuthOutcome {
    #[prost(string, tag = "1")]
    Completed(String),
    #[prost(bool, tag = "2")]
    Cancelled(bool),
    #[prost(string, tag = "3")]
    Failed(String),
}

impl From<&AuthResult> for AuthResultProto {
    fn from(r: &AuthResult) -> Self {
        let outcome = match r {
            AuthResult::Completed { username } => AuthOutcome::Completed(username.clone()),
            AuthResult::Cancelled => AuthOutcome::Cancelled(true),
            AuthResult::Failed { reason } => AuthOutcome::Failed(reason.clone()),
        };
        AuthResultProto {
            outcome: Some(outcome),
        }
    }
}

impl TryFrom<AuthResultProto> for AuthResult {
    type Error = WireError;

    fn try_from(p: AuthResultProto) -> Result<Self, WireError> {
        match p.outcome {
            Some(AuthOutcome::Completed(username)) => Ok(AuthResult::Completed { username }),
            Some(AuthOutcome::Cancelled(_)) => Ok(AuthResult::Cancelled),
            Some(AuthOutcome::Failed(reason)) => Ok(AuthResult::Failed { reason }),
            None => Err(WireError::MissingOutcome),
        }
    }
}

/// Sent from `credprovider` to `cef-host` over the control pipe to request
/// that the browser window close without completing the flow.
#[derive(Clone, Copy, PartialEq, prost::Message)]
pub struct CancelSignal {}

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
            WireError::MissingOutcome => write!(f, "AuthResult frame had no outcome set"),
        }
    }
}

impl std::error::Error for WireError {}

/// Frames larger than this are refused; every real message here is a short
/// username/reason string, so this only guards against a corrupt length
/// prefix turning into an unbounded allocation.
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
/// Returns `Ok(None)` if `r` is at EOF before any bytes of a new frame
/// arrive (the pipe's write end closed cleanly, e.g. the writer process
/// exited without sending a result) — callers treat that the same as an
/// explicit cancellation.
pub fn read_frame<T: prost::Message + Default, R: Read>(
    r: &mut R,
) -> Result<Option<T>, WireError> {
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

/// Write an `AuthResult` over the result pipe.
pub fn write_auth_result<W: Write>(w: &mut W, result: &AuthResult) -> Result<(), WireError> {
    write_frame(w, &AuthResultProto::from(result))
}

/// Read an `AuthResult` from the result pipe. See [`read_frame`] for EOF
/// handling.
pub fn read_auth_result<R: Read>(r: &mut R) -> Result<Option<AuthResult>, WireError> {
    match read_frame::<AuthResultProto, R>(r)? {
        Some(proto) => Ok(Some(AuthResult::try_from(proto)?)),
        None => Ok(None),
    }
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

/// Fixed size of the sign-in window, matching the current CEF window.
pub const WINDOW_WIDTH: i32 = 560;
pub const WINDOW_HEIGHT: i32 = 670;

/// `redirect_uri` prefix the sign-in flow completes on.
pub const REDIRECT_PREFIX: &str = "goauthentik.io://";

/// Header carrying the interactive-auth session token, injected on every
/// request the sign-in window makes.
pub const AUTH_HEADER_NAME: &str = "X-Authentik-Platform-Auth-DTH";

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_completed_through_a_stream() {
        let msg = AuthResult::Completed {
            username: "jdoe".to_string(),
        };
        let mut buf = Vec::new();
        write_auth_result(&mut buf, &msg).unwrap();

        let mut cursor = io::Cursor::new(buf);
        let decoded = read_auth_result(&mut cursor).unwrap().unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trips_cancelled_and_failed() {
        for msg in [
            AuthResult::Cancelled,
            AuthResult::Failed {
                reason: "token validation failed".to_string(),
            },
        ] {
            let mut buf = Vec::new();
            write_auth_result(&mut buf, &msg).unwrap();
            let mut cursor = io::Cursor::new(buf);
            let decoded = read_auth_result(&mut cursor).unwrap().unwrap();
            assert_eq!(msg, decoded);
        }
    }

    #[test]
    fn read_frame_reports_clean_eof_as_none() {
        let mut cursor = io::Cursor::new(Vec::<u8>::new());
        let decoded = read_auth_result(&mut cursor).unwrap();
        assert!(decoded.is_none());
    }

    #[test]
    fn read_frame_rejects_oversized_length_prefix() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(MAX_FRAME_BYTES + 1).to_le_bytes());
        let mut cursor = io::Cursor::new(buf);
        let result = read_auth_result(&mut cursor);
        assert!(matches!(result, Err(WireError::FrameTooLarge(_))));
    }

    #[test]
    fn cancel_signal_round_trips() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &CancelSignal {}).unwrap();
        let mut cursor = io::Cursor::new(buf);
        let decoded: Option<CancelSignal> = read_frame(&mut cursor).unwrap();
        assert_eq!(decoded, Some(CancelSignal {}));
    }

    #[test]
    fn tile_fields_match_current_appearance() {
        assert_eq!(TILE_FIELDS.len(), 4);
        assert_eq!(TILE_FIELDS[2].text, "Sign in with authentik");
        assert_eq!(TILE_FIELDS[3].text, "Submit");
    }
}
