// @generated
impl serde::Serialize for StateSession {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.username.is_empty() {
            len += 1;
        }
        if !self.token_hash.is_empty() {
            len += 1;
        }
        if self.expires_at.is_some() {
            len += 1;
        }
        if self.pid != 0 {
            len += 1;
        }
        if self.ppid != 0 {
            len += 1;
        }
        if self.created_at.is_some() {
            len += 1;
        }
        if !self.local_socket.is_empty() {
            len += 1;
        }
        if self.opened {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("state_sysd_session.StateSession", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if !self.username.is_empty() {
            struct_ser.serialize_field("username", &self.username)?;
        }
        if !self.token_hash.is_empty() {
            struct_ser.serialize_field("tokenHash", &self.token_hash)?;
        }
        if let Some(v) = self.expires_at.as_ref() {
            struct_ser.serialize_field("expiresAt", v)?;
        }
        if self.pid != 0 {
            struct_ser.serialize_field("pid", &self.pid)?;
        }
        if self.ppid != 0 {
            struct_ser.serialize_field("ppid", &self.ppid)?;
        }
        if let Some(v) = self.created_at.as_ref() {
            struct_ser.serialize_field("createdAt", v)?;
        }
        if !self.local_socket.is_empty() {
            struct_ser.serialize_field("localSocket", &self.local_socket)?;
        }
        if self.opened {
            struct_ser.serialize_field("opened", &self.opened)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateSession {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "username",
            "token_hash",
            "tokenHash",
            "expires_at",
            "expiresAt",
            "pid",
            "ppid",
            "created_at",
            "createdAt",
            "local_socket",
            "localSocket",
            "opened",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Username,
            TokenHash,
            ExpiresAt,
            Pid,
            Ppid,
            CreatedAt,
            LocalSocket,
            Opened,
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
                            "id" => Ok(GeneratedField::Id),
                            "username" => Ok(GeneratedField::Username),
                            "tokenHash" | "token_hash" => Ok(GeneratedField::TokenHash),
                            "expiresAt" | "expires_at" => Ok(GeneratedField::ExpiresAt),
                            "pid" => Ok(GeneratedField::Pid),
                            "ppid" => Ok(GeneratedField::Ppid),
                            "createdAt" | "created_at" => Ok(GeneratedField::CreatedAt),
                            "localSocket" | "local_socket" => Ok(GeneratedField::LocalSocket),
                            "opened" => Ok(GeneratedField::Opened),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StateSession;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct state_sysd_session.StateSession")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StateSession, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut username__ = None;
                let mut token_hash__ = None;
                let mut expires_at__ = None;
                let mut pid__ = None;
                let mut ppid__ = None;
                let mut created_at__ = None;
                let mut local_socket__ = None;
                let mut opened__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Username => {
                            if username__.is_some() {
                                return Err(serde::de::Error::duplicate_field("username"));
                            }
                            username__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TokenHash => {
                            if token_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenHash"));
                            }
                            token_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ExpiresAt => {
                            if expires_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expiresAt"));
                            }
                            expires_at__ = map_.next_value()?;
                        }
                        GeneratedField::Pid => {
                            if pid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pid"));
                            }
                            pid__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Ppid => {
                            if ppid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ppid"));
                            }
                            ppid__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CreatedAt => {
                            if created_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createdAt"));
                            }
                            created_at__ = map_.next_value()?;
                        }
                        GeneratedField::LocalSocket => {
                            if local_socket__.is_some() {
                                return Err(serde::de::Error::duplicate_field("localSocket"));
                            }
                            local_socket__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Opened => {
                            if opened__.is_some() {
                                return Err(serde::de::Error::duplicate_field("opened"));
                            }
                            opened__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StateSession {
                    id: id__.unwrap_or_default(),
                    username: username__.unwrap_or_default(),
                    token_hash: token_hash__.unwrap_or_default(),
                    expires_at: expires_at__,
                    pid: pid__.unwrap_or_default(),
                    ppid: ppid__.unwrap_or_default(),
                    created_at: created_at__,
                    local_socket: local_socket__.unwrap_or_default(),
                    opened: opened__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("state_sysd_session.StateSession", FIELDS, GeneratedVisitor)
    }
}
