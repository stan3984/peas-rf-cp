use flexi_logger::{LogSpecification, Logger};
use log::LevelFilter;

/// change the environment variable RUST_LOG to change the log level
pub fn initialize_logger(level: LevelFilter, to_file: bool) {
    Logger::with(LogSpecification::default(level).build())
        .o_log_to_file(to_file)
        .format(flexi_logger::opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
}
