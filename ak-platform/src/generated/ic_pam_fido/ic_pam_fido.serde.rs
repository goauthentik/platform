// @generated
impl serde::Serialize for FidoRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rp_id.is_empty() {
            len += 1;
        }
        if !self.challenge.is_empty() {
            len += 1;
        }
        if !self.credential_ids.is_empty() {
            len += 1;
        }
        if self.uv {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("ic_pam_fido.FIDORequest", len)?;
        if !self.rp_id.is_empty() {
            struct_ser.serialize_field("rpId", &self.rp_id)?;
        }
        if !self.challenge.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("challenge", pbjson::private::base64::encode(&self.challenge).as_str())?;
        }
        if !self.credential_ids.is_empty() {
            struct_ser.serialize_field("credentialIds", &self.credential_ids.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if self.uv {
            struct_ser.serialize_field("uv", &self.uv)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FidoRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rp_id",
            "rpId",
            "challenge",
            "credential_ids",
            "credentialIds",
            "uv",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RpId,
            Challenge,
            CredentialIds,
            Uv,
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
                            "rpId" | "rp_id" => Ok(GeneratedField::RpId),
                            "challenge" => Ok(GeneratedField::Challenge),
                            "credentialIds" | "credential_ids" => Ok(GeneratedField::CredentialIds),
                            "uv" => Ok(GeneratedField::Uv),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FidoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct ic_pam_fido.FIDORequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FidoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rp_id__ = None;
                let mut challenge__ = None;
                let mut credential_ids__ = None;
                let mut uv__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RpId => {
                            if rp_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rpId"));
                            }
                            rp_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Challenge => {
                            if challenge__.is_some() {
                                return Err(serde::de::Error::duplicate_field("challenge"));
                            }
                            challenge__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CredentialIds => {
                            if credential_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("credentialIds"));
                            }
                            credential_ids__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::Uv => {
                            if uv__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uv"));
                            }
                            uv__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(FidoRequest {
                    rp_id: rp_id__.unwrap_or_default(),
                    challenge: challenge__.unwrap_or_default(),
                    credential_ids: credential_ids__.unwrap_or_default(),
                    uv: uv__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("ic_pam_fido.FIDORequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FidoResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.credential_id.is_empty() {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        if !self.authenticator_data.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("ic_pam_fido.FIDOResponse", len)?;
        if !self.credential_id.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("credentialId", pbjson::private::base64::encode(&self.credential_id).as_str())?;
        }
        if !self.signature.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        if !self.authenticator_data.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("authenticatorData", pbjson::private::base64::encode(&self.authenticator_data).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FidoResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "credential_id",
            "credentialId",
            "signature",
            "authenticator_data",
            "authenticatorData",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CredentialId,
            Signature,
            AuthenticatorData,
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
                            "credentialId" | "credential_id" => Ok(GeneratedField::CredentialId),
                            "signature" => Ok(GeneratedField::Signature),
                            "authenticatorData" | "authenticator_data" => Ok(GeneratedField::AuthenticatorData),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FidoResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct ic_pam_fido.FIDOResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FidoResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut credential_id__ = None;
                let mut signature__ = None;
                let mut authenticator_data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CredentialId => {
                            if credential_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("credentialId"));
                            }
                            credential_id__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::AuthenticatorData => {
                            if authenticator_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authenticatorData"));
                            }
                            authenticator_data__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(FidoResponse {
                    credential_id: credential_id__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                    authenticator_data: authenticator_data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("ic_pam_fido.FIDOResponse", FIELDS, GeneratedVisitor)
    }
}
