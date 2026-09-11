use log::Log;

pub(crate) struct VoidLogger {}

impl VoidLogger {
    pub fn new() -> Box<Self> {
        Self {}.into()
    }
}

impl Log for VoidLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _record: &log::Record) {}

    fn flush(&self) {}
}
