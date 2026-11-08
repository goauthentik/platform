// @generated
impl serde::Serialize for PlatformEndpointRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.challenge.is_empty() {
            len += 1;
        }
        if !self.profile.is_empty() {
            len += 1;
        }
        if !self.agent_socket.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_platform.PlatformEndpointRequest", len)?;
        if !self.challenge.is_empty() {
            struct_ser.serialize_field("challenge", &self.challenge)?;
        }
        if !self.profile.is_empty() {
            struct_ser.serialize_field("profile", &self.profile)?;
        }
        if !self.agent_socket.is_empty() {
            struct_ser.serialize_field("agentSocket", &self.agent_socket)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlatformEndpointRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "challenge",
            "profile",
            "agent_socket",
            "agentSocket",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Challenge,
            Profile,
            AgentSocket,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "challenge" => Ok(GeneratedField::Challenge),
                            "profile" => Ok(GeneratedField::Profile),
                            "agentSocket" | "agent_socket" => Ok(GeneratedField::AgentSocket),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlatformEndpointRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_platform.PlatformEndpointRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlatformEndpointRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut challenge__ = None;
                let mut profile__ = None;
                let mut agent_socket__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Challenge => {
                            if challenge__.is_some() {
                                return Err(serde::de::Error::duplicate_field("challenge"));
                            }
                            challenge__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Profile => {
                            if profile__.is_some() {
                                return Err(serde::de::Error::duplicate_field("profile"));
                            }
                            profile__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AgentSocket => {
                            if agent_socket__.is_some() {
                                return Err(serde::de::Error::duplicate_field("agentSocket"));
                            }
                            agent_socket__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlatformEndpointRequest {
                    challenge: challenge__.unwrap_or_default(),
                    profile: profile__.unwrap_or_default(),
                    agent_socket: agent_socket__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_platform.PlatformEndpointRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlatformEndpointResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.header.is_some() {
            len += 1;
        }
        if !self.message.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_platform.PlatformEndpointResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.message.is_empty() {
            struct_ser.serialize_field("message", &self.message)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlatformEndpointResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "message",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Message,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "header" => Ok(GeneratedField::Header),
                            "message" => Ok(GeneratedField::Message),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlatformEndpointResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_platform.PlatformEndpointResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlatformEndpointResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut message__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlatformEndpointResponse {
                    header: header__,
                    message: message__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_platform.PlatformEndpointResponse", FIELDS, GeneratedVisitor)
    }
}
