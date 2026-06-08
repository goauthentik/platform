// @generated
impl serde::Serialize for Domain {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_ctrl.Domain", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Domain {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
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
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Domain;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_ctrl.Domain")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Domain, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Domain {
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_ctrl.Domain", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DomainEnrollRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.authentik_url.is_empty() {
            len += 1;
        }
        if !self.token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_ctrl.DomainEnrollRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.authentik_url.is_empty() {
            struct_ser.serialize_field("authentikUrl", &self.authentik_url)?;
        }
        if !self.token.is_empty() {
            struct_ser.serialize_field("token", &self.token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DomainEnrollRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "authentik_url",
            "authentikUrl",
            "token",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            AuthentikUrl,
            Token,
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
                            "name" => Ok(GeneratedField::Name),
                            "authentikUrl" | "authentik_url" => Ok(GeneratedField::AuthentikUrl),
                            "token" => Ok(GeneratedField::Token),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DomainEnrollRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_ctrl.DomainEnrollRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DomainEnrollRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut authentik_url__ = None;
                let mut token__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AuthentikUrl => {
                            if authentik_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authentikUrl"));
                            }
                            authentik_url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DomainEnrollRequest {
                    name: name__.unwrap_or_default(),
                    authentik_url: authentik_url__.unwrap_or_default(),
                    token: token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_ctrl.DomainEnrollRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DomainEnrollResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.device_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_ctrl.DomainEnrollResponse", len)?;
        if !self.device_id.is_empty() {
            struct_ser.serialize_field("deviceId", &self.device_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DomainEnrollResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "device_id",
            "deviceId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DeviceId,
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
                            "deviceId" | "device_id" => Ok(GeneratedField::DeviceId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DomainEnrollResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_ctrl.DomainEnrollResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DomainEnrollResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut device_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DeviceId => {
                            if device_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deviceId"));
                            }
                            device_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DomainEnrollResponse {
                    device_id: device_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_ctrl.DomainEnrollResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DomainListResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.domains.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_ctrl.DomainListResponse", len)?;
        if !self.domains.is_empty() {
            struct_ser.serialize_field("domains", &self.domains)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DomainListResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "domains",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Domains,
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
                            "domains" => Ok(GeneratedField::Domains),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DomainListResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_ctrl.DomainListResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DomainListResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut domains__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Domains => {
                            if domains__.is_some() {
                                return Err(serde::de::Error::duplicate_field("domains"));
                            }
                            domains__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DomainListResponse {
                    domains: domains__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_ctrl.DomainListResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TroubleshootInspectResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.bucket.is_empty() {
            len += 1;
        }
        if !self.kv.is_empty() {
            len += 1;
        }
        if !self.children.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_ctrl.TroubleshootInspectResponse", len)?;
        if !self.bucket.is_empty() {
            struct_ser.serialize_field("bucket", &self.bucket)?;
        }
        if !self.kv.is_empty() {
            struct_ser.serialize_field("kv", &self.kv)?;
        }
        if !self.children.is_empty() {
            struct_ser.serialize_field("children", &self.children)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TroubleshootInspectResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bucket",
            "kv",
            "children",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bucket,
            Kv,
            Children,
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
                            "bucket" => Ok(GeneratedField::Bucket),
                            "kv" => Ok(GeneratedField::Kv),
                            "children" => Ok(GeneratedField::Children),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TroubleshootInspectResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_ctrl.TroubleshootInspectResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TroubleshootInspectResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bucket__ = None;
                let mut kv__ = None;
                let mut children__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Bucket => {
                            if bucket__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bucket"));
                            }
                            bucket__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Kv => {
                            if kv__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kv"));
                            }
                            kv__ = Some(
                                map_.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::Children => {
                            if children__.is_some() {
                                return Err(serde::de::Error::duplicate_field("children"));
                            }
                            children__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TroubleshootInspectResponse {
                    bucket: bucket__.unwrap_or_default(),
                    kv: kv__.unwrap_or_default(),
                    children: children__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_ctrl.TroubleshootInspectResponse", FIELDS, GeneratedVisitor)
    }
}
