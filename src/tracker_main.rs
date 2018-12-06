extern crate peas_rf_cp;
use peas_rf_cp::tracker::server;

extern crate log;
use log::LevelFilter;

fn main() {
    peas_rf_cp::common::logger::initialize_logger(LevelFilter::Debug, false);
    server::start(12345, 600);
}
