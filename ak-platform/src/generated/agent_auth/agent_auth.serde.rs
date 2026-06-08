// @generated
impl serde::Serialize for AuthorizeRequest {
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
        if !self.service.is_empty() {
            len += 1;
        }
        if !self.uid.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.AuthorizeRequest", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.service.is_empty() {
            struct_ser.serialize_field("service", &self.service)?;
        }
        if !self.uid.is_empty() {
            struct_ser.serialize_field("uid", &self.uid)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AuthorizeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "service",
            "uid",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Service,
            Uid,
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
                            "header" => Ok(GeneratedField::Header),
                            "service" => Ok(GeneratedField::Service),
                            "uid" => Ok(GeneratedField::Uid),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AuthorizeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.AuthorizeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AuthorizeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut service__ = None;
                let mut uid__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Service => {
                            if service__.is_some() {
                                return Err(serde::de::Error::duplicate_field("service"));
                            }
                            service__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Uid => {
                            if uid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uid"));
                            }
                            uid__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AuthorizeRequest {
                    header: header__,
                    service: service__.unwrap_or_default(),
                    uid: uid__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.AuthorizeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AuthorizeResponse {
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
        let mut struct_ser = serializer.serialize_struct("agent_auth.AuthorizeResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AuthorizeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
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
                            "header" => Ok(GeneratedField::Header),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AuthorizeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.AuthorizeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AuthorizeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AuthorizeResponse {
                    header: header__,
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.AuthorizeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CurrentTokenRequest {
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
        if self.r#type != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.CurrentTokenRequest", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if self.r#type != 0 {
            let v = current_token_request::Type::try_from(self.r#type)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrentTokenRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "type",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Type,
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
                            "header" => Ok(GeneratedField::Header),
                            "type" => Ok(GeneratedField::Type),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CurrentTokenRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.CurrentTokenRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrentTokenRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut r#type__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map_.next_value::<current_token_request::Type>()? as i32);
                        }
                    }
                }
                Ok(CurrentTokenRequest {
                    header: header__,
                    r#type: r#type__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.CurrentTokenRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for current_token_request::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Verified => "VERIFIED",
            Self::Unverified => "UNVERIFIED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for current_token_request::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "UNSPECIFIED",
            "VERIFIED",
            "UNVERIFIED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = current_token_request::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "UNSPECIFIED" => Ok(current_token_request::Type::Unspecified),
                    "VERIFIED" => Ok(current_token_request::Type::Verified),
                    "UNVERIFIED" => Ok(current_token_request::Type::Unverified),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for CurrentTokenResponse {
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
        if self.token.is_some() {
            len += 1;
        }
        if !self.raw.is_empty() {
            len += 1;
        }
        if !self.url.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.CurrentTokenResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if let Some(v) = self.token.as_ref() {
            struct_ser.serialize_field("token", v)?;
        }
        if !self.raw.is_empty() {
            struct_ser.serialize_field("raw", &self.raw)?;
        }
        if !self.url.is_empty() {
            struct_ser.serialize_field("url", &self.url)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrentTokenResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "token",
            "raw",
            "url",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Token,
            Raw,
            Url,
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
                            "header" => Ok(GeneratedField::Header),
                            "token" => Ok(GeneratedField::Token),
                            "raw" => Ok(GeneratedField::Raw),
                            "url" => Ok(GeneratedField::Url),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CurrentTokenResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.CurrentTokenResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrentTokenResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut token__ = None;
                let mut raw__ = None;
                let mut url__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = map_.next_value()?;
                        }
                        GeneratedField::Raw => {
                            if raw__.is_some() {
                                return Err(serde::de::Error::duplicate_field("raw"));
                            }
                            raw__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Url => {
                            if url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("url"));
                            }
                            url__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CurrentTokenResponse {
                    header: header__,
                    token: token__,
                    raw: raw__.unwrap_or_default(),
                    url: url__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.CurrentTokenResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeviceTokenExchangeRequest {
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
        if !self.device_name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.DeviceTokenExchangeRequest", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.device_name.is_empty() {
            struct_ser.serialize_field("deviceName", &self.device_name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeviceTokenExchangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "device_name",
            "deviceName",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            DeviceName,
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
                            "header" => Ok(GeneratedField::Header),
                            "deviceName" | "device_name" => Ok(GeneratedField::DeviceName),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeviceTokenExchangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.DeviceTokenExchangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DeviceTokenExchangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut device_name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::DeviceName => {
                            if device_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deviceName"));
                            }
                            device_name__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DeviceTokenExchangeRequest {
                    header: header__,
                    device_name: device_name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.DeviceTokenExchangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenExchangeRequest {
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
        if !self.client_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.TokenExchangeRequest", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.client_id.is_empty() {
            struct_ser.serialize_field("clientId", &self.client_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenExchangeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "client_id",
            "clientId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            ClientId,
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
                            "header" => Ok(GeneratedField::Header),
                            "clientId" | "client_id" => Ok(GeneratedField::ClientId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TokenExchangeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.TokenExchangeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TokenExchangeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut client_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::ClientId => {
                            if client_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clientId"));
                            }
                            client_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TokenExchangeRequest {
                    header: header__,
                    client_id: client_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.TokenExchangeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenExchangeResponse {
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
        if !self.access_token.is_empty() {
            len += 1;
        }
        if self.expires_in != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.TokenExchangeResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.access_token.is_empty() {
            struct_ser.serialize_field("accessToken", &self.access_token)?;
        }
        if self.expires_in != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("expiresIn", ToString::to_string(&self.expires_in).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenExchangeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "access_token",
            "accessToken",
            "expires_in",
            "expiresIn",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            AccessToken,
            ExpiresIn,
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
                            "header" => Ok(GeneratedField::Header),
                            "accessToken" | "access_token" => Ok(GeneratedField::AccessToken),
                            "expiresIn" | "expires_in" => Ok(GeneratedField::ExpiresIn),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TokenExchangeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.TokenExchangeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TokenExchangeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut access_token__ = None;
                let mut expires_in__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::AccessToken => {
                            if access_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accessToken"));
                            }
                            access_token__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ExpiresIn => {
                            if expires_in__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expiresIn"));
                            }
                            expires_in__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TokenExchangeResponse {
                    header: header__,
                    access_token: access_token__.unwrap_or_default(),
                    expires_in: expires_in__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.TokenExchangeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WhoAmIRequest {
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
        let mut struct_ser = serializer.serialize_struct("agent_auth.WhoAmIRequest", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WhoAmIRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
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
                            "header" => Ok(GeneratedField::Header),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WhoAmIRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.WhoAmIRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<WhoAmIRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                    }
                }
                Ok(WhoAmIRequest {
                    header: header__,
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.WhoAmIRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WhoAmIResponse {
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
        if !self.body.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_auth.WhoAmIResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.body.is_empty() {
            struct_ser.serialize_field("body", &self.body)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WhoAmIResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "body",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Body,
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
                            "header" => Ok(GeneratedField::Header),
                            "body" => Ok(GeneratedField::Body),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WhoAmIResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_auth.WhoAmIResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<WhoAmIResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut body__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Body => {
                            if body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("body"));
                            }
                            body__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(WhoAmIResponse {
                    header: header__,
                    body: body__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_auth.WhoAmIResponse", FIELDS, GeneratedVisitor)
    }
}
