
extern crate peas_rf_cp;

use std::net::{UdpSocket,SocketAddr};
use std::env;
use peas_rf_cp::tracker::server;

fn main() {
    peas_rf_cp::common::logger::init_stderr();
    server::start(12345, 600);
}
