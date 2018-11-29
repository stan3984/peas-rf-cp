
extern crate peas_rf_cp;

use peas_rf_cp::common::logger;
use peas_rf_cp::tracker::api;
use peas_rf_cp::network::{self,NetworkError};
use std::env;
use std::net::ToSocketAddrs;
use peas_rf_cp::common::id::Id;

fn main() {
    logger::init_file();
    // log::info!("Hello, world!");

    // uppdatera oss själva för att sedan lista alla som tracker vet om

    let args: Vec<_> = env::args().collect();
    let sock = network::udp::open_any().unwrap();
    let tracker = args[1].to_socket_addrs().unwrap().next().unwrap();

    println!("tracker: {:?}", tracker);
    println!("me: {:?}", sock.local_addr().unwrap());

    // update test_id on tracker
    let test_id = Id::from_u64(12);
    println!("updating {}", test_id);
    let dur = match api::update(&sock, test_id, sock.local_addr().unwrap(), tracker) {
        Err(NetworkError::Timeout) => {
            println!("tracker not responding :(");
            std::process::exit(1);
        },
        Err(_) => {
            println!("shshzhshshszzhzzsh");
            std::process::exit(1);
        },
        Ok(ok) => ok,
    };
    println!("{} should now stay there for {}s", test_id, dur.as_secs());

    // querying everything
    let ls = api::LookupSession::new(&sock, tracker, test_id);

    for cur in ls {
        match cur {
            Err(NetworkError::Timeout) => {
                println!("tracker not responding");
            },
            Err(_) => {
                println!("ööööh");
                return;
            },
            Ok(adr) => {
                println!("{} knows about {}", adr, test_id);
            }
        }
    }

    // log::debug!("Shutting down application...");
}
