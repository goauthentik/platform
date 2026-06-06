// @generated
impl serde::Serialize for SshTokenAuthentication {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.token.is_empty() {
            len += 1;
        }
        if !self.local_socket.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("ic_ssh.SSHTokenAuthentication", len)?;
        if !self.token.is_empty() {
            struct_ser.serialize_field("token", &self.token)?;
        }
        if !self.local_socket.is_empty() {
            struct_ser.serialize_field("localSocket", &self.local_socket)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SshTokenAuthentication {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "token",
            "local_socket",
            "localSocket",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Token,
            LocalSocket,
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
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "token" => Ok(GeneratedField::Token),
                            "localSocket" | "local_socket" => Ok(GeneratedField::LocalSocket),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SshTokenAuthentication;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct ic_ssh.SSHTokenAuthentication")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SshTokenAuthentication, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut token__ = None;
                let mut local_socket__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = Some(map_.next_value()?);
                        }
                        GeneratedField::LocalSocket => {
                            if local_socket__.is_some() {
                                return Err(serde::de::Error::duplicate_field("localSocket"));
                            }
                            local_socket__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SshTokenAuthentication {
                    token: token__.unwrap_or_default(),
                    local_socket: local_socket__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("ic_ssh.SSHTokenAuthentication", FIELDS, GeneratedVisitor)
    }
}
