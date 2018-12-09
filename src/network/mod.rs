use pnet::datalink::interfaces;
use rand::Rng;
use std::io::{self, Read};
use std::net::{IpAddr, IpAddr::V4, Ipv4Addr, SocketAddr};
use std::{error::Error, fmt};

pub mod udp;
pub mod tcp;
pub mod udpmanager;

const MAX_UDP: usize = 512;
pub type Result<T> = std::result::Result<T, NetworkError>;

/// random network error
#[derive(Debug)]
pub enum NetworkError {
    IOError(io::Error),
    NoMessage,
    Timeout,
    Other(&'static str),
}

impl Error for NetworkError {
    fn description(&self) -> &str {
        match *self {
            NetworkError::IOError(ref e) => e.description(),
            NetworkError::NoMessage => "the received packet was not meant for us",
            NetworkError::Timeout => "the receiving timed out",
            NetworkError::Other(ref s) => s,
            // _ => "some random network error",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            NetworkError::IOError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NetworkError::IOError(ref e) => e.fmt(f),
            _ => write!(f, "NetworkError: {}", self.description()),
        }
    }
}

impl From<io::Error> for NetworkError {
    fn from(error: io::Error) -> Self {
        NetworkError::IOError(error)
    }
}

// TODO: cache?
/// finds the first best `Ipv4Addr` to use
pub fn find_internet_interface() -> Result<Ipv4Addr> {
    let ifaces = interfaces();
    for i in ifaces.iter() {
        if !i.is_loopback() && i.is_up() {
            for adrs in i.ips.iter() {
                if let V4(ip4) = adrs.ip() {
                    return Ok(ip4);
                }
            }
        }
    }
    Err(NetworkError::Other("couldn't find an available interface"))
}

// /// returns `num` connection candidates, consisting of `adr` and a random port.
// pub fn get_connection_candidates(adr: Ipv4Addr, num: i32) -> Vec<SocketAddr> {
//     let mut rng = rand::thread_rng();
//     let mut res = Vec::new();

//     for _ in 1..num {
//         res.push(SocketAddr::new(IpAddr::from(adr), rng.gen_range(1024, u16::max_value())));
//     }
//     res
// }

/// creates a SocketAddr from an ipv4 address and port
pub fn from_ipv4(adr: Ipv4Addr, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::from(adr), port)
}
