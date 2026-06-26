use std::{fmt::Debug, future::Future};

use serde::{Serialize, de::DeserializeOwned};

use crate::prelude::*;

pub trait Config: Default + Serialize + DeserializeOwned + Sized + Sync + Send + Debug + Clone {
    fn post_load(&mut self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }
    fn pre_save(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }
    fn post_update(&self, _prev: Self) -> impl Future<Output = Result<ConfigChangedType>> + Send {
        async { Ok(ConfigChangedType::Generic) }
    }
}

pub enum ConfigChangedType {
    Generic,
    Added,
    Removed,
}
