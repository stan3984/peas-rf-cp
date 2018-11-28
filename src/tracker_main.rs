
extern crate peas_rf_cp;

use std::net::{UdpSocket,SocketAddr};
use std::env;
use peas_rf_cp::tracker::server;

fn main() {
    server::start(12345, 600);
}
