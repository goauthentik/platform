pub const EXT_AUTHENTIK_AGENT_TUNNEL: &str = "agent-tunnel@goauthentik.io";

/// Payload sent to the SSH agent via the tunnel extension.
/// `method` is the gRPC path (e.g. `/package.Service/Method`).
/// `data` is the raw serialized proto request (no gRPC framing).
pub struct ExtAuthentikAgentTunnelData {
    pub method: String,
    pub data: Vec<u8>,
}

impl ExtAuthentikAgentTunnelData {
    pub fn serialize(&self) -> Vec<u8> {
        let method = self.method.as_bytes();
        let mut buf = Vec::with_capacity(8 + method.len() + self.data.len());
        buf.extend_from_slice(&(method.len() as u32).to_be_bytes());
        buf.extend_from_slice(method);
        buf.extend_from_slice(&(self.data.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        if buf.len() < 4 {
            return None;
        }
        let method_len = u32::from_be_bytes(buf[0..4].try_into().ok()?) as usize;
        let method_start = 4;
        let method_end = method_start + method_len;
        if buf.len() < method_end + 4 {
            return None;
        }
        let method = String::from_utf8(buf[method_start..method_end].to_vec()).ok()?;

        let data_len =
            u32::from_be_bytes(buf[method_end..method_end + 4].try_into().ok()?) as usize;

        let data_start = method_end + 4;
        let data_end = data_start + data_len;
        if buf.len() < data_end {
            return None;
        }
        let data = buf[data_start..data_end].to_vec();

        Some(Self { method, data })
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
}
