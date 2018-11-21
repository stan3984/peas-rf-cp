
use std::net::{UdpSocket,SocketAddr};
use common::id::Id;
use std::collections::HashMap;
use std::time::{Duration,SystemTime};
use super::{TrackResp,TrackQuery};
use bincode::{serialize, deserialize};
use network::udp;

/// maps room ids to several entry (bootstrap) nodes
struct Data(HashMap<Id, Vec<Boot>>);

/// data stored in the tracker
struct Boot {
    /// address to a entry node
    adr: SocketAddr,
    /// the time the entry was added
    ttl: SystemTime,
    /// strictly increasing counter to act as an id for every boot
    counter: u32,
}

impl PartialEq for Boot {
    fn eq(&self, other: &Boot) -> bool {
        self.adr == other.adr
    }
}

impl Boot {
    /// add a new boot
    fn new(adr: SocketAddr, counter: &mut u32) -> Boot {
        *counter += 1;
        Boot {adr: adr, ttl: SystemTime::now(), counter: *counter}
    }
}

impl Data {
    fn new() -> Data {
        Data(HashMap::new())
    }

    /// add `adr` as an bootstrap node for a room with id `id`.
    /// if `adr` already exists for `id`, then update the ttl for it.
    /// `counter` is a global variable for ids.
    fn update(&mut self, counter: &mut u32, id: Id, adr: SocketAddr) {
        let data = &mut self.0;
        if !data.contains_key(&id) {
            data.insert(id, vec![Boot::new(adr, counter)]);
        } else if let Some(ref mut x) = data.get_mut(&id) {

            let mut contained = false;
            for ele in x.iter_mut() {
                if ele.adr == adr {
                    ele.ttl = SystemTime::now();
                    contained = true;
                    break;
                }
            }

            if ! contained {
                x.push(Boot::new(adr, counter));
            }
        }
    }

    /// find the address and counter for the next bootstrap node for room with id `id`.
    /// `counter` is the counter of the previously looked up node for room `id`,
    /// This makes sure that we aren't returning the same node twice.
    /// If this is the first lookup for `id`, then use `counter` = 0.
    /// If there aren't any nodes left in the "database", then `Nonde` is returned.
    fn lookup(&self, id: Id, counter: u32) -> Option<(SocketAddr, u32)> {
        if let Some(ref x) = self.0.get(&id) {
            for ele in x.iter() {
                if ele.counter > counter {
                    return Some((ele.adr, ele.counter))
                }
            }
        }
        None
    }

    /// remove everything older than `thres`
    fn remove_old(&mut self, thres: Duration, now: SystemTime) {
        self.0.retain(|_, ref mut val| {
            val.retain(|ref boot| {
                if let Ok(dur) = now.duration_since(boot.ttl) {
                    dur <= thres
                } else {
                    // there isn't really anything sensible to do if
                    // `duration_since` fails, so just ignore it and
                    // keep the entry
                    true
                }
            });
            !val.is_empty()
        });
    }
}

/// start_from_tup(([127,0,0,1], 8080))
pub fn start_from_tup(address: ([u8; 4], u16)) {
    start(SocketAddr::from(address));
}

// TODO: bara ge port och hitta locala addressen själv med network::find_internet_interface?
pub fn start(address: SocketAddr) {
    let mut data = Data::new();
    let mut counter: u32 = 0;

    let sock = UdpSocket::bind(address).expect("couldn't bind socket");

    loop {
        let (sender, query): (_, TrackQuery) = udp::recv_until(&sock).unwrap();
        // handle the request
        udp::send(&sock, &TrackResp::LookupAns{adr: None, lookup_id: 2}, sender).unwrap();
    }

}


