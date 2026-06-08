// @generated
impl serde::Serialize for RegisterDeviceRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.device_signing_key.is_empty() {
            len += 1;
        }
        if !self.device_encryption_key.is_empty() {
            len += 1;
        }
        if !self.enc_key_id.is_empty() {
            len += 1;
        }
        if !self.sign_key_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth_apple.RegisterDeviceRequest", len)?;
        if !self.device_signing_key.is_empty() {
            struct_ser.serialize_field("deviceSigningKey", &self.device_signing_key)?;
        }
        if !self.device_encryption_key.is_empty() {
            struct_ser.serialize_field("deviceEncryptionKey", &self.device_encryption_key)?;
        }
        if !self.enc_key_id.is_empty() {
            struct_ser.serialize_field("encKeyId", &self.enc_key_id)?;
        }
        if !self.sign_key_id.is_empty() {
            struct_ser.serialize_field("signKeyId", &self.sign_key_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RegisterDeviceRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "device_signing_key",
            "deviceSigningKey",
            "device_encryption_key",
            "deviceEncryptionKey",
            "enc_key_id",
            "encKeyId",
            "sign_key_id",
            "signKeyId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DeviceSigningKey,
            DeviceEncryptionKey,
            EncKeyId,
            SignKeyId,
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
                            "deviceSigningKey" | "device_signing_key" => Ok(GeneratedField::DeviceSigningKey),
                            "deviceEncryptionKey" | "device_encryption_key" => Ok(GeneratedField::DeviceEncryptionKey),
                            "encKeyId" | "enc_key_id" => Ok(GeneratedField::EncKeyId),
                            "signKeyId" | "sign_key_id" => Ok(GeneratedField::SignKeyId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RegisterDeviceRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth_apple.RegisterDeviceRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RegisterDeviceRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut device_signing_key__ = None;
                let mut device_encryption_key__ = None;
                let mut enc_key_id__ = None;
                let mut sign_key_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DeviceSigningKey => {
                            if device_signing_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deviceSigningKey"));
                            }
                            device_signing_key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DeviceEncryptionKey => {
                            if device_encryption_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deviceEncryptionKey"));
                            }
                            device_encryption_key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::EncKeyId => {
                            if enc_key_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("encKeyId"));
                            }
                            enc_key_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SignKeyId => {
                            if sign_key_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signKeyId"));
                            }
                            sign_key_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RegisterDeviceRequest {
                    device_signing_key: device_signing_key__.unwrap_or_default(),
                    device_encryption_key: device_encryption_key__.unwrap_or_default(),
                    enc_key_id: enc_key_id__.unwrap_or_default(),
                    sign_key_id: sign_key_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth_apple.RegisterDeviceRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RegisterDeviceResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.client_id.is_empty() {
            len += 1;
        }
        if !self.issuer.is_empty() {
            len += 1;
        }
        if !self.token_endpoint.is_empty() {
            len += 1;
        }
        if !self.jwks_endpoint.is_empty() {
            len += 1;
        }
        if !self.audience.is_empty() {
            len += 1;
        }
        if !self.nonce_endpoint.is_empty() {
            len += 1;
        }
        if !self.device_token.is_empty() {
            len += 1;
        }
        if !self.authorization_endpoint.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth_apple.RegisterDeviceResponse", len)?;
        if !self.client_id.is_empty() {
            struct_ser.serialize_field("clientId", &self.client_id)?;
        }
        if !self.issuer.is_empty() {
            struct_ser.serialize_field("issuer", &self.issuer)?;
        }
        if !self.token_endpoint.is_empty() {
            struct_ser.serialize_field("tokenEndpoint", &self.token_endpoint)?;
        }
        if !self.jwks_endpoint.is_empty() {
            struct_ser.serialize_field("jwksEndpoint", &self.jwks_endpoint)?;
        }
        if !self.audience.is_empty() {
            struct_ser.serialize_field("audience", &self.audience)?;
        }
        if !self.nonce_endpoint.is_empty() {
            struct_ser.serialize_field("nonceEndpoint", &self.nonce_endpoint)?;
        }
        if !self.device_token.is_empty() {
            struct_ser.serialize_field("deviceToken", &self.device_token)?;
        }
        if !self.authorization_endpoint.is_empty() {
            struct_ser.serialize_field("authorizationEndpoint", &self.authorization_endpoint)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RegisterDeviceResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "client_id",
            "clientId",
            "issuer",
            "token_endpoint",
            "tokenEndpoint",
            "jwks_endpoint",
            "jwksEndpoint",
            "audience",
            "nonce_endpoint",
            "nonceEndpoint",
            "device_token",
            "deviceToken",
            "authorization_endpoint",
            "authorizationEndpoint",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ClientId,
            Issuer,
            TokenEndpoint,
            JwksEndpoint,
            Audience,
            NonceEndpoint,
            DeviceToken,
            AuthorizationEndpoint,
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
                            "clientId" | "client_id" => Ok(GeneratedField::ClientId),
                            "issuer" => Ok(GeneratedField::Issuer),
                            "tokenEndpoint" | "token_endpoint" => Ok(GeneratedField::TokenEndpoint),
                            "jwksEndpoint" | "jwks_endpoint" => Ok(GeneratedField::JwksEndpoint),
                            "audience" => Ok(GeneratedField::Audience),
                            "nonceEndpoint" | "nonce_endpoint" => Ok(GeneratedField::NonceEndpoint),
                            "deviceToken" | "device_token" => Ok(GeneratedField::DeviceToken),
                            "authorizationEndpoint" | "authorization_endpoint" => Ok(GeneratedField::AuthorizationEndpoint),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RegisterDeviceResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth_apple.RegisterDeviceResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RegisterDeviceResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut client_id__ = None;
                let mut issuer__ = None;
                let mut token_endpoint__ = None;
                let mut jwks_endpoint__ = None;
                let mut audience__ = None;
                let mut nonce_endpoint__ = None;
                let mut device_token__ = None;
                let mut authorization_endpoint__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ClientId => {
                            if client_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clientId"));
                            }
                            client_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Issuer => {
                            if issuer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("issuer"));
                            }
                            issuer__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TokenEndpoint => {
                            if token_endpoint__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenEndpoint"));
                            }
                            token_endpoint__ = Some(map_.next_value()?);
                        }
                        GeneratedField::JwksEndpoint => {
                            if jwks_endpoint__.is_some() {
                                return Err(serde::de::Error::duplicate_field("jwksEndpoint"));
                            }
                            jwks_endpoint__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Audience => {
                            if audience__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audience"));
                            }
                            audience__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NonceEndpoint => {
                            if nonce_endpoint__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonceEndpoint"));
                            }
                            nonce_endpoint__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DeviceToken => {
                            if device_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deviceToken"));
                            }
                            device_token__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AuthorizationEndpoint => {
                            if authorization_endpoint__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authorizationEndpoint"));
                            }
                            authorization_endpoint__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RegisterDeviceResponse {
                    client_id: client_id__.unwrap_or_default(),
                    issuer: issuer__.unwrap_or_default(),
                    token_endpoint: token_endpoint__.unwrap_or_default(),
                    jwks_endpoint: jwks_endpoint__.unwrap_or_default(),
                    audience: audience__.unwrap_or_default(),
                    nonce_endpoint: nonce_endpoint__.unwrap_or_default(),
                    device_token: device_token__.unwrap_or_default(),
                    authorization_endpoint: authorization_endpoint__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth_apple.RegisterDeviceResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RegisterUserRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.user_secure_enclave_key.is_empty() {
            len += 1;
        }
        if !self.enclave_key_id.is_empty() {
            len += 1;
        }
        if !self.user_auth.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth_apple.RegisterUserRequest", len)?;
        if !self.user_secure_enclave_key.is_empty() {
            struct_ser.serialize_field("userSecureEnclaveKey", &self.user_secure_enclave_key)?;
        }
        if !self.enclave_key_id.is_empty() {
            struct_ser.serialize_field("enclaveKeyId", &self.enclave_key_id)?;
        }
        if !self.user_auth.is_empty() {
            struct_ser.serialize_field("userAuth", &self.user_auth)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RegisterUserRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "user_secure_enclave_key",
            "userSecureEnclaveKey",
            "enclave_key_id",
            "enclaveKeyId",
            "user_auth",
            "userAuth",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            UserSecureEnclaveKey,
            EnclaveKeyId,
            UserAuth,
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
                            "userSecureEnclaveKey" | "user_secure_enclave_key" => Ok(GeneratedField::UserSecureEnclaveKey),
                            "enclaveKeyId" | "enclave_key_id" => Ok(GeneratedField::EnclaveKeyId),
                            "userAuth" | "user_auth" => Ok(GeneratedField::UserAuth),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RegisterUserRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth_apple.RegisterUserRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RegisterUserRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut user_secure_enclave_key__ = None;
                let mut enclave_key_id__ = None;
                let mut user_auth__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::UserSecureEnclaveKey => {
                            if user_secure_enclave_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("userSecureEnclaveKey"));
                            }
                            user_secure_enclave_key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::EnclaveKeyId => {
                            if enclave_key_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enclaveKeyId"));
                            }
                            enclave_key_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::UserAuth => {
                            if user_auth__.is_some() {
                                return Err(serde::de::Error::duplicate_field("userAuth"));
                            }
                            user_auth__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RegisterUserRequest {
                    user_secure_enclave_key: user_secure_enclave_key__.unwrap_or_default(),
                    enclave_key_id: enclave_key_id__.unwrap_or_default(),
                    user_auth: user_auth__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth_apple.RegisterUserRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RegisterUserResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.username.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth_apple.RegisterUserResponse", len)?;
        if !self.username.is_empty() {
            struct_ser.serialize_field("username", &self.username)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RegisterUserResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "username",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Username,
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
                            "username" => Ok(GeneratedField::Username),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RegisterUserResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth_apple.RegisterUserResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RegisterUserResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut username__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Username => {
                            if username__.is_some() {
                                return Err(serde::de::Error::duplicate_field("username"));
                            }
                            username__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RegisterUserResponse {
                    username: username__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth_apple.RegisterUserResponse", FIELDS, GeneratedVisitor)
    }
}
