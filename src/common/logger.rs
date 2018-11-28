
use flexi_logger::Logger;

/// log ONLY to a file
pub fn init_file() {
    init_file_stderr_level(true);
}

/// log ONLY to stderr
pub fn init_stderr() {
    init_file_stderr_level(false);
}

/// change the environment variable RUST_LOG to change the log level
pub fn init_file_stderr_level(file_or_stderr: bool) {
    Logger::with_env_or_str("info")
        .o_log_to_file(file_or_stderr)
        .format(flexi_logger::opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
}


// /// wrapper around expect that also logs as error
// pub fn log_expect<T, E>(res: Result<T, E>, msg: &str) -> T
// where E: std::fmt::Debug,
// {
//     res.map_err(|_| {log::error!("{:?}", msg);}).expect(msg)
// }
