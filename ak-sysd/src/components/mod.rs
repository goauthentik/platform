use ak_platform::{paths::SysdSocketID, prelude::*};
use std::sync::Arc;

pub mod auth;
pub mod ping;

pub type ComponentConstructor = dyn Fn() -> Result<dyn Component>;

pub trait Component {
    fn new() -> Result<impl Component> where Self: Sized;
    fn id() -> String where Self: Sized;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn register_for_id(&self, id: SysdSocketID);
}

pub struct ComponentInstance {
    comp: Arc<Box<dyn Component>>,
}
