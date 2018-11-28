#![allow(dead_code)]

extern crate log;
extern crate rand;
extern crate simple_logging;

mod common;
mod node;

use common::logger;

fn main() {
    logger::init(logger::LevelFilter::Debug);
    log::debug!("Hello, world!");

    // magic goes here

    log::debug!("Shutting down application...");
}
