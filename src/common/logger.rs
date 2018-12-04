use flexi_logger::{LogSpecification, Logger};
use log::{self, LevelFilter};

pub fn initialize_logger(level: LevelFilter, to_file: bool) {
    if level != LevelFilter::Off {
        let logger = Logger::with(LogSpecification::default(level).build())
            .o_log_to_file(to_file)
            .format(flexi_logger::opt_format)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

        log::info!("Logger initialized (filter level: {})", level);
    }
}
