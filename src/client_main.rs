
extern crate peas_rf_cp;

use peas_rf_cp::common::logger;

fn main() {
    logger::init(logger::LevelFilter::Debug);
    log::debug!("Hello, world!");

    println!("main started");
    // magic goes here

    log::debug!("Shutting down application...");
}
