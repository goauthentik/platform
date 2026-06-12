use serde::{Serialize, de::DeserializeOwned};

use crate::prelude::*;

pub trait Config: Default + Serialize + DeserializeOwned + Sized + Sync + Send {
    fn post_load(&self) -> Result<()> {
        Ok(())
    }
    fn pre_save(&self) -> Result<()> {
        Ok(())
    }
    fn post_update(&self, _prev: Self) -> Result<ConfigChangedType> {
        Ok(ConfigChangedType::Generic)
    }
}

pub enum ConfigChangedType {
    Generic,
    Added,
    Removed,
}
