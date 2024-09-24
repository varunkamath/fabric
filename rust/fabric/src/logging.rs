use log::{Level, LevelFilter, Metadata, Record};
use std::sync::Once;

pub struct FabricLogger;

impl log::Log for FabricLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static INIT: Once = Once::new();

pub fn init_logger(level: LevelFilter) {
    INIT.call_once(|| {
        let logger = FabricLogger;
        log::set_boxed_logger(Box::new(logger)).unwrap();
        log::set_max_level(level);
    });
}
