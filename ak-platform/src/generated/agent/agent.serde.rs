// @generated
impl serde::Serialize for RequestHeader {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.profile.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent.RequestHeader", len)?;
        if !self.profile.is_empty() {
            struct_ser.serialize_field("profile", &self.profile)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RequestHeader {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "profile",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Profile,
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
                            "profile" => Ok(GeneratedField::Profile),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RequestHeader;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent.RequestHeader")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RequestHeader, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut profile__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Profile => {
                            if profile__.is_some() {
                                return Err(serde::de::Error::duplicate_field("profile"));
                            }
                            profile__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RequestHeader {
                    profile: profile__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent.RequestHeader", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ResponseHeader {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.successful {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent.ResponseHeader", len)?;
        if self.successful {
            struct_ser.serialize_field("successful", &self.successful)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ResponseHeader {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "successful",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Successful,
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
                            "successful" => Ok(GeneratedField::Successful),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ResponseHeader;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent.ResponseHeader")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ResponseHeader, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut successful__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Successful => {
                            if successful__.is_some() {
                                return Err(serde::de::Error::duplicate_field("successful"));
                            }
                            successful__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ResponseHeader {
                    successful: successful__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent.ResponseHeader", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Token {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.preferred_username.is_empty() {
            len += 1;
        }
        if !self.iss.is_empty() {
            len += 1;
        }
        if !self.sub.is_empty() {
            len += 1;
        }
        if !self.aud.is_empty() {
            len += 1;
        }
        if self.exp.is_some() {
            len += 1;
        }
        if self.nbf.is_some() {
            len += 1;
        }
        if self.iat.is_some() {
            len += 1;
        }
        if !self.jti.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent.Token", len)?;
        if !self.preferred_username.is_empty() {
            struct_ser.serialize_field("preferredUsername", &self.preferred_username)?;
        }
        if !self.iss.is_empty() {
            struct_ser.serialize_field("iss", &self.iss)?;
        }
        if !self.sub.is_empty() {
            struct_ser.serialize_field("sub", &self.sub)?;
        }
        if !self.aud.is_empty() {
            struct_ser.serialize_field("aud", &self.aud)?;
        }
        if let Some(v) = self.exp.as_ref() {
            struct_ser.serialize_field("exp", v)?;
        }
        if let Some(v) = self.nbf.as_ref() {
            struct_ser.serialize_field("nbf", v)?;
        }
        if let Some(v) = self.iat.as_ref() {
            struct_ser.serialize_field("iat", v)?;
        }
        if !self.jti.is_empty() {
            struct_ser.serialize_field("jti", &self.jti)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Token {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "preferred_username",
            "preferredUsername",
            "iss",
            "sub",
            "aud",
            "exp",
            "nbf",
            "iat",
            "jti",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PreferredUsername,
            Iss,
            Sub,
            Aud,
            Exp,
            Nbf,
            Iat,
            Jti,
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
                            "preferredUsername" | "preferred_username" => Ok(GeneratedField::PreferredUsername),
                            "iss" => Ok(GeneratedField::Iss),
                            "sub" => Ok(GeneratedField::Sub),
                            "aud" => Ok(GeneratedField::Aud),
                            "exp" => Ok(GeneratedField::Exp),
                            "nbf" => Ok(GeneratedField::Nbf),
                            "iat" => Ok(GeneratedField::Iat),
                            "jti" => Ok(GeneratedField::Jti),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Token;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent.Token")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Token, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut preferred_username__ = None;
                let mut iss__ = None;
                let mut sub__ = None;
                let mut aud__ = None;
                let mut exp__ = None;
                let mut nbf__ = None;
                let mut iat__ = None;
                let mut jti__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PreferredUsername => {
                            if preferred_username__.is_some() {
                                return Err(serde::de::Error::duplicate_field("preferredUsername"));
                            }
                            preferred_username__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Iss => {
                            if iss__.is_some() {
                                return Err(serde::de::Error::duplicate_field("iss"));
                            }
                            iss__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Sub => {
                            if sub__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sub"));
                            }
                            sub__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Aud => {
                            if aud__.is_some() {
                                return Err(serde::de::Error::duplicate_field("aud"));
                            }
                            aud__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Exp => {
                            if exp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("exp"));
                            }
                            exp__ = map_.next_value()?;
                        }
                        GeneratedField::Nbf => {
                            if nbf__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nbf"));
                            }
                            nbf__ = map_.next_value()?;
                        }
                        GeneratedField::Iat => {
                            if iat__.is_some() {
                                return Err(serde::de::Error::duplicate_field("iat"));
                            }
                            iat__ = map_.next_value()?;
                        }
                        GeneratedField::Jti => {
                            if jti__.is_some() {
                                return Err(serde::de::Error::duplicate_field("jti"));
                            }
                            jti__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Token {
                    preferred_username: preferred_username__.unwrap_or_default(),
                    iss: iss__.unwrap_or_default(),
                    sub: sub__.unwrap_or_default(),
                    aud: aud__.unwrap_or_default(),
                    exp: exp__,
                    nbf: nbf__,
                    iat: iat__,
                    jti: jti__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent.Token", FIELDS, GeneratedVisitor)
    }
}
