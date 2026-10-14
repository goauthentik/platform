pub const EXT_AUTHENTIK_AGENT_TUNNEL: &str = "agent-tunnel@goauthentik.io";

/// Type tag the agent prepends to its tunnel response, mirroring
/// `SSH_AGENT_EXTENSION_RESPONSE`. The SSH agent protocol has already consumed its
/// own copy of this byte by the time the extension payload reaches us, so this one
/// belongs to the payload.
pub const SSH_AGENT_EXT_RESPONSE_TYPE: u8 = 29;

/// Payload sent to the SSH agent via the tunnel extension.
/// `method` is the full gRPC path (e.g. `/package.Service/Method`), which is what
/// `MethodCaller` expects — it goes straight into an HTTP request URI on the agent
/// side, and a path without the leading slash is not a valid URI.
/// `data` is the raw serialized proto request (no gRPC framing).
pub struct ExtAuthentikAgentTunnelData {
    pub method: String,
    pub data: Vec<u8>,
}

impl ExtAuthentikAgentTunnelData {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + self.method.len() + self.data.len());
        push_field(&mut buf, self.method.as_bytes());
        push_field(&mut buf, &self.data);
        buf
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        let mut fields = Fields::new(buf);
        Some(Self {
            method: fields.string()?,
            data: fields.field()?.to_vec(),
        })
    }
}

/// Payload the SSH agent sends back through the tunnel extension.
///
/// Wire: `[u8 type][string method][bytes data][u32 grpc-status][string grpc-message]`.
///
/// The two status fields are a later addition, so they are optional on the way in: an
/// agent that predates them stops after `data`, which reads back as status 0 with no
/// message. They matter because the agent has no other channel for a failure — before
/// they existed every error, including an ordinary non-OK gRPC status, came back as an
/// empty payload that the client could only report as "empty tunnel response".
pub struct ExtAuthentikAgentTunnelResponse {
    pub method: String,
    pub data: Vec<u8>,
    /// gRPC status code, as `tonic::Code as i32`. Zero means OK.
    pub status: i32,
    pub message: String,
}

impl ExtAuthentikAgentTunnelResponse {
    pub fn ok(method: String, data: Vec<u8>) -> Self {
        Self {
            method,
            data,
            status: 0,
            message: String::new(),
        }
    }

    /// A failure with no response body. `method` may be empty when the request could
    /// not be decoded far enough to know it.
    pub fn error(method: String, status: i32, message: impl Into<String>) -> Self {
        Self {
            method,
            data: Vec::new(),
            status,
            message: message.into(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf =
            Vec::with_capacity(17 + self.method.len() + self.data.len() + self.message.len());
        buf.push(SSH_AGENT_EXT_RESPONSE_TYPE);
        push_field(&mut buf, self.method.as_bytes());
        push_field(&mut buf, &self.data);
        buf.extend_from_slice(&(self.status as u32).to_be_bytes());
        push_field(&mut buf, self.message.as_bytes());
        buf
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        let (tag, rest) = buf.split_first()?;
        if *tag != SSH_AGENT_EXT_RESPONSE_TYPE {
            return None;
        }

        let mut fields = Fields::new(rest);
        let method = fields.string()?;
        let data = fields.field()?.to_vec();

        let (status, message) = if fields.is_empty() {
            (0, String::new())
        } else {
            (fields.u32()? as i32, fields.string()?)
        };

        Some(Self {
            method,
            data,
            status,
            message,
        })
    }
}

fn push_field(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buf.extend_from_slice(data);
}

/// Cursor over the `u32`-length-prefixed fields both payloads are built from, matching
/// the SSH encoding the agent side uses via `ssh_encoding::Encode`.
struct Fields<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Fields<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn u32(&mut self) -> Option<u32> {
        let raw = self.buf.get(self.pos..self.pos.checked_add(4)?)?;
        self.pos += 4;
        Some(u32::from_be_bytes(raw.try_into().ok()?))
    }

    fn field(&mut self) -> Option<&'a [u8]> {
        let len = self.u32()? as usize;
        let raw = self.buf.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(raw)
    }

    fn string(&mut self) -> Option<String> {
        String::from_utf8(self.field()?.to_vec()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::{agent::RequestHeader, agent_auth::WhoAmIRequest};
    use prost::Message;

    #[test]
    fn serialize() {
        let msg = WhoAmIRequest {
            header: Some(RequestHeader {
                profile: "default".to_string(),
            }),
        };
        let encoded = msg.encode_to_vec();
        let ext = ExtAuthentikAgentTunnelData {
            method: "ping.Ping/Ping".to_string(),
            data: encoded,
        };
        assert_eq!(
            ext.serialize(),
            [
                0, 0, 0, 14, 112, 105, 110, 103, 46, 80, 105, 110, 103, 47, 80, 105, 110, 103, 0,
                0, 0, 11, 10, 9, 10, 7, 100, 101, 102, 97, 117, 108, 116
            ]
        );
    }

    #[test]
    fn deserialize() {
        let encoded: Vec<u8> = vec![
            0, 0, 0, 14, 112, 105, 110, 103, 46, 80, 105, 110, 103, 47, 80, 105, 110, 103, 0, 0, 0,
            11, 10, 9, 10, 7, 100, 101, 102, 97, 117, 108, 116,
        ];
        let parsed = ExtAuthentikAgentTunnelData::deserialize(&encoded).unwrap();
        assert_eq!(parsed.method, "ping.Ping/Ping");

        let m = WhoAmIRequest::decode(&*parsed.data).unwrap();

        assert_eq!(m.header.unwrap().profile, "default");
    }

    #[test]
    fn deserialize_truncated_returns_none() {
        assert!(ExtAuthentikAgentTunnelData::deserialize(&[0, 0, 0, 14, 112]).is_none());
        assert!(ExtAuthentikAgentTunnelData::deserialize(&[0, 0, 0, 1]).is_none());
        assert!(ExtAuthentikAgentTunnelData::deserialize(&[]).is_none());
    }

    #[test]
    fn deserialize_empty_data_field() {
        let ext = ExtAuthentikAgentTunnelData {
            method: "/ping.Ping/Ping".to_string(),
            data: Vec::new(),
        };
        let parsed = ExtAuthentikAgentTunnelData::deserialize(&ext.serialize()).unwrap();
        assert_eq!(parsed.method, "/ping.Ping/Ping");
        assert!(parsed.data.is_empty());
    }

    // --- response ---

    #[test]
    fn response_roundtrip_ok() {
        let res =
            ExtAuthentikAgentTunnelResponse::ok("/ping.Ping/Ping".to_string(), vec![1, 2, 3, 4, 5]);
        let parsed =
            ExtAuthentikAgentTunnelResponse::deserialize(&res.serialize()).expect("must parse");
        assert_eq!(parsed.method, "/ping.Ping/Ping");
        assert_eq!(parsed.data, vec![1, 2, 3, 4, 5]);
        assert_eq!(parsed.status, 0);
        assert_eq!(parsed.message, "");
    }

    #[test]
    fn response_roundtrip_error() {
        let res = ExtAuthentikAgentTunnelResponse::error(
            "/ping.Ping/Ping".to_string(),
            5,
            "no such user",
        );
        let parsed =
            ExtAuthentikAgentTunnelResponse::deserialize(&res.serialize()).expect("must parse");
        assert!(parsed.data.is_empty());
        assert_eq!(parsed.status, 5);
        assert_eq!(parsed.message, "no such user");
    }

    /// An agent built before the status fields existed stops after `data`. That has to
    /// keep parsing, as OK, or every older agent breaks the moment the client updates.
    #[test]
    fn response_without_status_fields_parses_as_ok() {
        let mut legacy = vec![SSH_AGENT_EXT_RESPONSE_TYPE];
        push_field(&mut legacy, b"/ping.Ping/Ping");
        push_field(&mut legacy, &[9, 9]);

        let parsed = ExtAuthentikAgentTunnelResponse::deserialize(&legacy).expect("must parse");
        assert_eq!(parsed.method, "/ping.Ping/Ping");
        assert_eq!(parsed.data, vec![9, 9]);
        assert_eq!(parsed.status, 0);
        assert_eq!(parsed.message, "");
    }

    #[test]
    fn response_rejects_empty_mistagged_and_truncated_payloads() {
        assert!(ExtAuthentikAgentTunnelResponse::deserialize(&[]).is_none());

        let res = ExtAuthentikAgentTunnelResponse::error("/a.B/C".to_string(), 5, "nope");

        let mut mistagged = res.serialize();
        mistagged[0] = SSH_AGENT_EXT_RESPONSE_TYPE + 1;
        assert!(ExtAuthentikAgentTunnelResponse::deserialize(&mistagged).is_none());

        let full = res.serialize();
        assert!(ExtAuthentikAgentTunnelResponse::deserialize(&full[..full.len() - 3]).is_none());
    }
}
