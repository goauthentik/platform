// @generated
impl serde::Serialize for ListProfilesResponse {
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
        if !self.profiles.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_ctrl.ListProfilesResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.profiles.is_empty() {
            struct_ser.serialize_field("profiles", &self.profiles)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListProfilesResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "profiles",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            Profiles,
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
                            "profiles" => Ok(GeneratedField::Profiles),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListProfilesResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_ctrl.ListProfilesResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ListProfilesResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut profiles__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::Profiles => {
                            if profiles__.is_some() {
                                return Err(serde::de::Error::duplicate_field("profiles"));
                            }
                            profiles__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ListProfilesResponse {
                    header: header__,
                    profiles: profiles__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_ctrl.ListProfilesResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Profile {
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
        if !self.username.is_empty() {
            len += 1;
        }
        if !self.authentik_url.is_empty() {
            len += 1;
        }
        if self.last_renewed.is_some() {
            len += 1;
        }
        if self.next_renew.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_ctrl.Profile", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.username.is_empty() {
            struct_ser.serialize_field("username", &self.username)?;
        }
        if !self.authentik_url.is_empty() {
            struct_ser.serialize_field("authentikUrl", &self.authentik_url)?;
        }
        if let Some(v) = self.last_renewed.as_ref() {
            struct_ser.serialize_field("lastRenewed", v)?;
        }
        if let Some(v) = self.next_renew.as_ref() {
            struct_ser.serialize_field("nextRenew", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Profile {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "username",
            "authentik_url",
            "authentikUrl",
            "last_renewed",
            "lastRenewed",
            "next_renew",
            "nextRenew",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Username,
            AuthentikUrl,
            LastRenewed,
            NextRenew,
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
                            "username" => Ok(GeneratedField::Username),
                            "authentikUrl" | "authentik_url" => Ok(GeneratedField::AuthentikUrl),
                            "lastRenewed" | "last_renewed" => Ok(GeneratedField::LastRenewed),
                            "nextRenew" | "next_renew" => Ok(GeneratedField::NextRenew),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Profile;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_ctrl.Profile")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Profile, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut username__ = None;
                let mut authentik_url__ = None;
                let mut last_renewed__ = None;
                let mut next_renew__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Username => {
                            if username__.is_some() {
                                return Err(serde::de::Error::duplicate_field("username"));
                            }
                            username__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AuthentikUrl => {
                            if authentik_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authentikUrl"));
                            }
                            authentik_url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::LastRenewed => {
                            if last_renewed__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastRenewed"));
                            }
                            last_renewed__ = map_.next_value()?;
                        }
                        GeneratedField::NextRenew => {
                            if next_renew__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextRenew"));
                            }
                            next_renew__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Profile {
                    name: name__.unwrap_or_default(),
                    username: username__.unwrap_or_default(),
                    authentik_url: authentik_url__.unwrap_or_default(),
                    last_renewed: last_renewed__,
                    next_renew: next_renew__,
                })
            }
        }
        deserializer.deserialize_struct("agent_ctrl.Profile", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetupRequest {
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
        if !self.authentik_url.is_empty() {
            len += 1;
        }
        if !self.app_slug.is_empty() {
            len += 1;
        }
        if !self.client_id.is_empty() {
            len += 1;
        }
        if !self.access_token.is_empty() {
            len += 1;
        }
        if !self.refresh_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("agent_ctrl.SetupRequest", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.authentik_url.is_empty() {
            struct_ser.serialize_field("authentikUrl", &self.authentik_url)?;
        }
        if !self.app_slug.is_empty() {
            struct_ser.serialize_field("appSlug", &self.app_slug)?;
        }
        if !self.client_id.is_empty() {
            struct_ser.serialize_field("clientId", &self.client_id)?;
        }
        if !self.access_token.is_empty() {
            struct_ser.serialize_field("accessToken", &self.access_token)?;
        }
        if !self.refresh_token.is_empty() {
            struct_ser.serialize_field("refreshToken", &self.refresh_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetupRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "authentik_url",
            "authentikUrl",
            "app_slug",
            "appSlug",
            "client_id",
            "clientId",
            "access_token",
            "accessToken",
            "refresh_token",
            "refreshToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            AuthentikUrl,
            AppSlug,
            ClientId,
            AccessToken,
            RefreshToken,
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
                            "authentikUrl" | "authentik_url" => Ok(GeneratedField::AuthentikUrl),
                            "appSlug" | "app_slug" => Ok(GeneratedField::AppSlug),
                            "clientId" | "client_id" => Ok(GeneratedField::ClientId),
                            "accessToken" | "access_token" => Ok(GeneratedField::AccessToken),
                            "refreshToken" | "refresh_token" => Ok(GeneratedField::RefreshToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetupRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_ctrl.SetupRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetupRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut authentik_url__ = None;
                let mut app_slug__ = None;
                let mut client_id__ = None;
                let mut access_token__ = None;
                let mut refresh_token__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::AuthentikUrl => {
                            if authentik_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authentikUrl"));
                            }
                            authentik_url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AppSlug => {
                            if app_slug__.is_some() {
                                return Err(serde::de::Error::duplicate_field("appSlug"));
                            }
                            app_slug__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ClientId => {
                            if client_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clientId"));
                            }
                            client_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AccessToken => {
                            if access_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accessToken"));
                            }
                            access_token__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RefreshToken => {
                            if refresh_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("refreshToken"));
                            }
                            refresh_token__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SetupRequest {
                    header: header__,
                    authentik_url: authentik_url__.unwrap_or_default(),
                    app_slug: app_slug__.unwrap_or_default(),
                    client_id: client_id__.unwrap_or_default(),
                    access_token: access_token__.unwrap_or_default(),
                    refresh_token: refresh_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("agent_ctrl.SetupRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetupResponse {
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
        let mut struct_ser = serializer.serialize_struct("agent_ctrl.SetupResponse", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetupResponse {
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
            type Value = SetupResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct agent_ctrl.SetupResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetupResponse, V::Error>
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
                Ok(SetupResponse {
                    header: header__,
                })
            }
        }
        deserializer.deserialize_struct("agent_ctrl.SetupResponse", FIELDS, GeneratedVisitor)
    }
}
