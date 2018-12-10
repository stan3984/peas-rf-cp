
extern crate peas_rf_cp;

use peas_rf_cp::common::logger;
use peas_rf_cp::tracker::api;
use peas_rf_cp::network::{self,NetworkError};
use peas_rf_cp::node::nethandle::NetHandle;
use std::env;
use std::net::ToSocketAddrs;
use peas_rf_cp::common::id::Id;
use peas_rf_cp::ui::cursive_main;
use::std::sync::{Arc, Mutex};

fn main() {
    println!("hej1");
    let net_new = Arc::new(Mutex::new(NetHandle::new(false,
                                                    Id::from_u64(5),
                                                    "stan3984".to_string(),
                                                    Id::from_u64(511),
                                                    vec![])));
    println!("hej2");
    cursive_main(net_new);
    println!("hej4");
}
