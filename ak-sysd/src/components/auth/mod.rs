use crate::components::Component;

pub struct AuthComponent {}

impl Component for AuthComponent {
    fn new() -> ak_platform::prelude::Result<Box<dyn Component>> {
        Ok(Box::new(Self {}))
    }

    fn start(&self) -> ak_platform::prelude::Result<()> {
        todo!()
    }

    fn stop(&self) -> ak_platform::prelude::Result<()> {
        todo!()
    }

    fn register_for_id(&self, _id: ak_platform::paths::SysdSocketID) {
        todo!()
    }

    fn id() -> String {
        "auth".to_string()
    }
}
