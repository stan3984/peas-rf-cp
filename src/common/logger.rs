use std::{io::Result, option::Option, sync::{self, Once}, time::SystemTime};
use simple_logging;

pub use log::LevelFilter;

static LOG_INIT_ONCE: Once = sync::ONCE_INIT;

/// Initializes a logger with a specified level filter.
///
/// Returns `Some` if this initialization has not been performed before, and `None` otherwise.
pub fn init(level: LevelFilter) -> Option<Result<()>> {
    let mut status = None;
    
    LOG_INIT_ONCE.call_once(|| {
        status = Some(init0(level));
    });

    status
}

fn init0(level: LevelFilter) -> Result<()> {
    let name = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(dur) => format!("peas-{:?}.log", dur.as_secs()),
        Err(_) => String::from("unknown")
    };
    
    simple_logging::log_to_file(name, level)
}
