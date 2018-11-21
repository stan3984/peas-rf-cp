use pnet::datalink::interfaces;
use std::net::{IpAddr::V4, Ipv4Addr, SocketAddr, IpAddr};
use std::{error::Error, fmt};
use rand::Rng;
use std::io::{self, Read};

pub mod udp;

/// random network error
// TODO: make more specific variants
#[derive(Debug)]
pub struct NetworkError;

impl Error for NetworkError {}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}

impl From<io::Error> for NetworkError {
    fn from(error: io::Error) -> Self {
       NetworkError
    }
}

// TODO: cache?
/// finds the first best `Ipv4Addr` to use
pub fn find_internet_interface() -> Result<Ipv4Addr, NetworkError> {
    let ifaces = interfaces();
    for i in ifaces.iter() {
        if ! i.is_loopback() && i.is_up() {
            for adrs in i.ips.iter() {
                if let V4(ip4) = adrs.ip() {
                    return Ok(ip4)
                }
            }
        }
    }
    Err(NetworkError)
}

/// returns `num` connection candidates, consisting of `adr` and a random port.
pub fn get_connection_candidates(adr: Ipv4Addr, num: i32) -> Vec<SocketAddr> {
    let mut rng = rand::thread_rng();
    let mut res = Vec::new();

    for _ in 1..num {
        res.push(SocketAddr::new(IpAddr::from(adr), rng.gen_range(1024, u16::max_value())));
    }
    res
}
