use crate::components::Component;
use eyre::Result;

pub struct AuthComponent {}

impl Component for AuthComponent {
    fn new() -> Result<Box<dyn Component>> {
        Ok(Box::new(Self {}))
    }

    fn start(&self) -> Result<()> {
        todo!()
    }

    fn stop(&self) -> Result<()> {
        todo!()
    }

    fn register_for_id(&self, _id: ak_platform::paths::SysdSocketID) {
        todo!()
    }

    fn id() -> String {
        "auth".to_string()
    }
}
