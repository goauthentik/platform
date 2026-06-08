// @generated
impl serde::Serialize for InteractiveAuthAsyncResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.url.is_empty() {
            len += 1;
        }
        if !self.header_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.InteractiveAuthAsyncResponse", len)?;
        if !self.url.is_empty() {
            struct_ser.serialize_field("url", &self.url)?;
        }
        if !self.header_token.is_empty() {
            struct_ser.serialize_field("headerToken", &self.header_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InteractiveAuthAsyncResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "url",
            "header_token",
            "headerToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Url,
            HeaderToken,
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
                            "url" => Ok(GeneratedField::Url),
                            "headerToken" | "header_token" => Ok(GeneratedField::HeaderToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InteractiveAuthAsyncResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.InteractiveAuthAsyncResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InteractiveAuthAsyncResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut url__ = None;
                let mut header_token__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Url => {
                            if url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("url"));
                            }
                            url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::HeaderToken => {
                            if header_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("headerToken"));
                            }
                            header_token__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(InteractiveAuthAsyncResponse {
                    url: url__.unwrap_or_default(),
                    header_token: header_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.InteractiveAuthAsyncResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for InteractiveAuthContinueRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.txid.is_empty() {
            len += 1;
        }
        if !self.value.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.InteractiveAuthContinueRequest", len)?;
        if !self.txid.is_empty() {
            struct_ser.serialize_field("txid", &self.txid)?;
        }
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InteractiveAuthContinueRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "txid",
            "value",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Txid,
            Value,
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
                            "txid" => Ok(GeneratedField::Txid),
                            "value" => Ok(GeneratedField::Value),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InteractiveAuthContinueRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.InteractiveAuthContinueRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InteractiveAuthContinueRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut txid__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Txid => {
                            if txid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txid"));
                            }
                            txid__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(InteractiveAuthContinueRequest {
                    txid: txid__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.InteractiveAuthContinueRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for InteractiveAuthInitRequest {
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
        if !self.password.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.InteractiveAuthInitRequest", len)?;
        if !self.username.is_empty() {
            struct_ser.serialize_field("username", &self.username)?;
        }
        if !self.password.is_empty() {
            struct_ser.serialize_field("password", &self.password)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InteractiveAuthInitRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "username",
            "password",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Username,
            Password,
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
                            "password" => Ok(GeneratedField::Password),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InteractiveAuthInitRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.InteractiveAuthInitRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InteractiveAuthInitRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut username__ = None;
                let mut password__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Username => {
                            if username__.is_some() {
                                return Err(serde::de::Error::duplicate_field("username"));
                            }
                            username__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Password => {
                            if password__.is_some() {
                                return Err(serde::de::Error::duplicate_field("password"));
                            }
                            password__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(InteractiveAuthInitRequest {
                    username: username__.unwrap_or_default(),
                    password: password__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.InteractiveAuthInitRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for InteractiveAuthRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.interactive_auth.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.InteractiveAuthRequest", len)?;
        if let Some(v) = self.interactive_auth.as_ref() {
            match v {
                interactive_auth_request::InteractiveAuth::Init(v) => {
                    struct_ser.serialize_field("init", v)?;
                }
                interactive_auth_request::InteractiveAuth::Continue(v) => {
                    struct_ser.serialize_field("continue", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InteractiveAuthRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "init",
            "continue",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Init,
            Continue,
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
                            "init" => Ok(GeneratedField::Init),
                            "continue" => Ok(GeneratedField::Continue),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InteractiveAuthRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.InteractiveAuthRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InteractiveAuthRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut interactive_auth__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Init => {
                            if interactive_auth__.is_some() {
                                return Err(serde::de::Error::duplicate_field("init"));
                            }
                            interactive_auth__ = map_.next_value::<::std::option::Option<_>>()?.map(interactive_auth_request::InteractiveAuth::Init)
;
                        }
                        GeneratedField::Continue => {
                            if interactive_auth__.is_some() {
                                return Err(serde::de::Error::duplicate_field("continue"));
                            }
                            interactive_auth__ = map_.next_value::<::std::option::Option<_>>()?.map(interactive_auth_request::InteractiveAuth::Continue)
;
                        }
                    }
                }
                Ok(InteractiveAuthRequest {
                    interactive_auth: interactive_auth__,
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.InteractiveAuthRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for InteractiveAuthResult {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::PamSuccess => "PAM_SUCCESS",
            Self::PamPermDenied => "PAM_PERM_DENIED",
            Self::PamAuthErr => "PAM_AUTH_ERR",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for InteractiveAuthResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "PAM_SUCCESS",
            "PAM_PERM_DENIED",
            "PAM_AUTH_ERR",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InteractiveAuthResult;

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
                    "PAM_SUCCESS" => Ok(InteractiveAuthResult::PamSuccess),
                    "PAM_PERM_DENIED" => Ok(InteractiveAuthResult::PamPermDenied),
                    "PAM_AUTH_ERR" => Ok(InteractiveAuthResult::PamAuthErr),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for InteractiveChallenge {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.txid.is_empty() {
            len += 1;
        }
        if self.finished {
            len += 1;
        }
        if self.result != 0 {
            len += 1;
        }
        if !self.prompt.is_empty() {
            len += 1;
        }
        if self.prompt_meta != 0 {
            len += 1;
        }
        if !self.debug_info.is_empty() {
            len += 1;
        }
        if !self.session_id.is_empty() {
            len += 1;
        }
        if !self.component.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.InteractiveChallenge", len)?;
        if !self.txid.is_empty() {
            struct_ser.serialize_field("txid", &self.txid)?;
        }
        if self.finished {
            struct_ser.serialize_field("finished", &self.finished)?;
        }
        if self.result != 0 {
            let v = InteractiveAuthResult::try_from(self.result)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.result)))?;
            struct_ser.serialize_field("result", &v)?;
        }
        if !self.prompt.is_empty() {
            struct_ser.serialize_field("prompt", &self.prompt)?;
        }
        if self.prompt_meta != 0 {
            let v = interactive_challenge::PromptMeta::try_from(self.prompt_meta)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.prompt_meta)))?;
            struct_ser.serialize_field("promptMeta", &v)?;
        }
        if !self.debug_info.is_empty() {
            struct_ser.serialize_field("debugInfo", &self.debug_info)?;
        }
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if !self.component.is_empty() {
            struct_ser.serialize_field("component", &self.component)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InteractiveChallenge {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "txid",
            "finished",
            "result",
            "prompt",
            "prompt_meta",
            "promptMeta",
            "debug_info",
            "debugInfo",
            "session_id",
            "sessionId",
            "component",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Txid,
            Finished,
            Result,
            Prompt,
            PromptMeta,
            DebugInfo,
            SessionId,
            Component,
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
                            "txid" => Ok(GeneratedField::Txid),
                            "finished" => Ok(GeneratedField::Finished),
                            "result" => Ok(GeneratedField::Result),
                            "prompt" => Ok(GeneratedField::Prompt),
                            "promptMeta" | "prompt_meta" => Ok(GeneratedField::PromptMeta),
                            "debugInfo" | "debug_info" => Ok(GeneratedField::DebugInfo),
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            "component" => Ok(GeneratedField::Component),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InteractiveChallenge;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.InteractiveChallenge")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InteractiveChallenge, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut txid__ = None;
                let mut finished__ = None;
                let mut result__ = None;
                let mut prompt__ = None;
                let mut prompt_meta__ = None;
                let mut debug_info__ = None;
                let mut session_id__ = None;
                let mut component__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Txid => {
                            if txid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txid"));
                            }
                            txid__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Finished => {
                            if finished__.is_some() {
                                return Err(serde::de::Error::duplicate_field("finished"));
                            }
                            finished__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Result => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result__ = Some(map_.next_value::<InteractiveAuthResult>()? as i32);
                        }
                        GeneratedField::Prompt => {
                            if prompt__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prompt"));
                            }
                            prompt__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PromptMeta => {
                            if prompt_meta__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptMeta"));
                            }
                            prompt_meta__ = Some(map_.next_value::<interactive_challenge::PromptMeta>()? as i32);
                        }
                        GeneratedField::DebugInfo => {
                            if debug_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("debugInfo"));
                            }
                            debug_info__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Component => {
                            if component__.is_some() {
                                return Err(serde::de::Error::duplicate_field("component"));
                            }
                            component__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(InteractiveChallenge {
                    txid: txid__.unwrap_or_default(),
                    finished: finished__.unwrap_or_default(),
                    result: result__.unwrap_or_default(),
                    prompt: prompt__.unwrap_or_default(),
                    prompt_meta: prompt_meta__.unwrap_or_default(),
                    debug_info: debug_info__.unwrap_or_default(),
                    session_id: session_id__.unwrap_or_default(),
                    component: component__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.InteractiveChallenge", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for interactive_challenge::PromptMeta {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::PamPromptEchoOff => "PAM_PROMPT_ECHO_OFF",
            Self::PamPromptEchoOn => "PAM_PROMPT_ECHO_ON",
            Self::PamErrorMsg => "PAM_ERROR_MSG",
            Self::PamTextInfo => "PAM_TEXT_INFO",
            Self::PamRadioType => "PAM_RADIO_TYPE",
            Self::PamBinaryPrompt => "PAM_BINARY_PROMPT",
            Self::Password => "PASSWORD",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for interactive_challenge::PromptMeta {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "UNSPECIFIED",
            "PAM_PROMPT_ECHO_OFF",
            "PAM_PROMPT_ECHO_ON",
            "PAM_ERROR_MSG",
            "PAM_TEXT_INFO",
            "PAM_RADIO_TYPE",
            "PAM_BINARY_PROMPT",
            "PASSWORD",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = interactive_challenge::PromptMeta;

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
                    "UNSPECIFIED" => Ok(interactive_challenge::PromptMeta::Unspecified),
                    "PAM_PROMPT_ECHO_OFF" => Ok(interactive_challenge::PromptMeta::PamPromptEchoOff),
                    "PAM_PROMPT_ECHO_ON" => Ok(interactive_challenge::PromptMeta::PamPromptEchoOn),
                    "PAM_ERROR_MSG" => Ok(interactive_challenge::PromptMeta::PamErrorMsg),
                    "PAM_TEXT_INFO" => Ok(interactive_challenge::PromptMeta::PamTextInfo),
                    "PAM_RADIO_TYPE" => Ok(interactive_challenge::PromptMeta::PamRadioType),
                    "PAM_BINARY_PROMPT" => Ok(interactive_challenge::PromptMeta::PamBinaryPrompt),
                    "PASSWORD" => Ok(interactive_challenge::PromptMeta::Password),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for SystemAuthorizeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.session_id.is_empty() {
            len += 1;
        }
        if self.authz.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.SystemAuthorizeRequest", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if let Some(v) = self.authz.as_ref() {
            struct_ser.serialize_field("authz", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SystemAuthorizeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "session_id",
            "sessionId",
            "authz",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
            Authz,
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
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            "authz" => Ok(GeneratedField::Authz),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SystemAuthorizeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.SystemAuthorizeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SystemAuthorizeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut session_id__ = None;
                let mut authz__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Authz => {
                            if authz__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authz"));
                            }
                            authz__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SystemAuthorizeRequest {
                    session_id: session_id__.unwrap_or_default(),
                    authz: authz__,
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.SystemAuthorizeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SystemAuthorizeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.response.is_some() {
            len += 1;
        }
        if self.code != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.SystemAuthorizeResponse", len)?;
        if let Some(v) = self.response.as_ref() {
            struct_ser.serialize_field("response", v)?;
        }
        if self.code != 0 {
            let v = InteractiveAuthResult::try_from(self.code)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.code)))?;
            struct_ser.serialize_field("code", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SystemAuthorizeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "response",
            "code",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Response,
            Code,
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
                            "response" => Ok(GeneratedField::Response),
                            "code" => Ok(GeneratedField::Code),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SystemAuthorizeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.SystemAuthorizeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SystemAuthorizeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut response__ = None;
                let mut code__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Response => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("response"));
                            }
                            response__ = map_.next_value()?;
                        }
                        GeneratedField::Code => {
                            if code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code__ = Some(map_.next_value::<InteractiveAuthResult>()? as i32);
                        }
                    }
                }
                Ok(SystemAuthorizeResponse {
                    response: response__,
                    code: code__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.SystemAuthorizeResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenAuthRequest {
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
        if !self.token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.TokenAuthRequest", len)?;
        if !self.username.is_empty() {
            struct_ser.serialize_field("username", &self.username)?;
        }
        if !self.token.is_empty() {
            struct_ser.serialize_field("token", &self.token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenAuthRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "username",
            "token",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Username,
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
                            "username" => Ok(GeneratedField::Username),
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
            type Value = TokenAuthRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.TokenAuthRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TokenAuthRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut username__ = None;
                let mut token__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Username => {
                            if username__.is_some() {
                                return Err(serde::de::Error::duplicate_field("username"));
                            }
                            username__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TokenAuthRequest {
                    username: username__.unwrap_or_default(),
                    token: token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.TokenAuthRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TokenAuthResponse {
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
        if self.token.is_some() {
            len += 1;
        }
        if !self.session_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("sys_auth.TokenAuthResponse", len)?;
        if self.successful {
            struct_ser.serialize_field("successful", &self.successful)?;
        }
        if let Some(v) = self.token.as_ref() {
            struct_ser.serialize_field("token", v)?;
        }
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TokenAuthResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "successful",
            "token",
            "session_id",
            "sessionId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Successful,
            Token,
            SessionId,
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
                            "token" => Ok(GeneratedField::Token),
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TokenAuthResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct sys_auth.TokenAuthResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TokenAuthResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut successful__ = None;
                let mut token__ = None;
                let mut session_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Successful => {
                            if successful__.is_some() {
                                return Err(serde::de::Error::duplicate_field("successful"));
                            }
                            successful__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = map_.next_value()?;
                        }
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TokenAuthResponse {
                    successful: successful__.unwrap_or_default(),
                    token: token__,
                    session_id: session_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("sys_auth.TokenAuthResponse", FIELDS, GeneratedVisitor)
    }
}
