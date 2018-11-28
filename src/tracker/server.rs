
use std::net::{IpAddr,UdpSocket,SocketAddr};
use common::id::Id;
use std::collections::HashMap;
use std::time::{Duration,Instant};
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
    ttl: Instant,
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
        Boot {adr: adr, ttl: Instant::now(), counter: *counter}
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
                    ele.ttl = Instant::now();
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
    /// If there aren't any nodes left in the "database", then `None` is returned.
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
    /// returns the systime of the oldest entry
    fn remove_old(&mut self, thres: Duration, now: Instant) -> Option<Instant> {
        let mut oldest = None;
        self.0.retain(|_, ref mut val| {
            val.retain(|ref boot| {
                let dur = now.duration_since(boot.ttl);
                if dur > thres {
                    false
                } else {
                    if oldest.is_none() || boot.ttl < oldest.unwrap() {
                        oldest = Some(boot.ttl);
                    }
                    true
                }
            });
            !val.is_empty()
        });
        oldest
    }

    fn length(&self) -> usize {
        let mut res = 0;
        for v in self.0.values() {
            res += v.len();
        }
        res
    }
}

pub fn start(port: u16, ttl: u64) {
    let mut data = Data::new();
    let mut counter: u32 = 0;
    let mut oldest_sys_time = Instant::now();
    let boot_ttl = Duration::from_secs(ttl);

    let my_ip = ::network::find_internet_interface().expect("couldn't find a suitable interface, are you even connected to a network?");
    let sock = UdpSocket::bind(SocketAddr::new(IpAddr::from(my_ip), port)).expect("couldn't bind socket, is the port already in use?");

    udp::set_blocking(&sock).unwrap();

    println!("Tracker started on {}:{} with entry ttl {}s", my_ip, port, ttl);

    loop {
        let (sender, query): (_, TrackQuery) = udp::recv_until_msg(&sock).unwrap();

        match query {
            TrackQuery::Update{id, adr} => {
                data.update(&mut counter, id, adr);
                println!("{} wants to update {}, counter is now {}", sender, id, counter);
                udp::send(&sock, &TrackResp::UpdateSuccess{id: id, ttl: boot_ttl}, sender).unwrap();
            }
            TrackQuery::Lookup{id, last_lookup} => {
                let (boot_adr, boot_cnt) = data.lookup(id, last_lookup).map_or((None, 0), |(a,c)| (Some(a),c));
                println!("{} wants to lookup {} with ll={}. We returned {:?} with ll={}", sender, id, last_lookup, boot_adr, boot_cnt);
                udp::send(&sock, &TrackResp::LookupAns{adr: boot_adr, lookup_id: boot_cnt}, sender).unwrap();
            }
        }

        let now = Instant::now();
        let dur = now.duration_since(oldest_sys_time);
        if dur > boot_ttl {
            let len_before = data.length();
            println!("removing old stuffs...");
            oldest_sys_time = data.remove_old(boot_ttl, now).unwrap_or(now);
            let len_after = data.length();
            println!("done! {} were removed, {} remain", len_before - len_after, len_after);
        }

    }
}


