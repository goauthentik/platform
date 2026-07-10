use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::sync::Arc;

pub mod auth;
pub mod ping;

pub type ComponentConstructor = fn() -> Result<Box<dyn Component>>;

pub trait Component {
    fn new() -> Result<Box<dyn Component>>
    where
        Self: Sized;
    fn id() -> String
    where
        Self: Sized;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn register_for_id(&self, id: SysdSocketID);
}

pub struct ComponentInstance {
    comp: Arc<Box<dyn Component>>,
}

impl ComponentInstance {
    pub fn new(comp: Box<dyn Component>) -> ComponentInstance {
        ComponentInstance {
            comp: Arc::new(comp),
        }
    }
}
