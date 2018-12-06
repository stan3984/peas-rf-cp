
use std::net::{UdpSocket, SocketAddr};
use ::network::{Result,NetworkError};
use ::network::udp;
use tracker::api;
use ::common::id::Id;
use std::time::Duration;
use ::node::ktable::{Entry,Ktable};
use std::sync::{Arc,Mutex};
use std::thread;
use std::collections::{HashMap,HashSet};
use std::sync::mpsc;

const LOOKUP_SIZE: usize = 5;
const K: usize = 3;

#[derive(Serialize, Deserialize, Debug)]
enum KadMsg {
    /// checks if another host is alive
    Ping,
    /// answer to `Ping`
    Pong(Id),
    /// requests id to be looked up
    /// Lookup(id_to_lookup, requester_entry)
    Lookup(Id, Entry),
    /// answer to lookup
    Answer(Vec<Entry>),
}

impl KadMsg {
    pub fn is_pong(&self) -> bool {
        if let KadMsg::Pong(_) = self {
            return true;
        }
        return false;
    }
    pub fn is_answer(&self) -> bool {
        if let KadMsg::Answer(_) = self {
            return true;
        }
        return false;
    }
}

/// creates a ktable in a mutex for cross thread use
pub fn create_ktable(my_id: Id) -> Arc<Mutex<Ktable>> {
    Arc::new(Mutex::new(Ktable::new(K as u32, my_id)))
}

/// starts a lookup of `id` in a separate thread. `ktable` is continously updated with new nodes and removal of dead ones.
/// returns the K nodes that are the closest to `id`
pub fn id_lookup(sock: UdpSocket, id: Id, myself: Entry, ktable: Arc<Mutex<Ktable>>) -> mpsc::Receiver<Vec<Entry>> {
    // TODO: clear socket or remove all old packages?
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut visited: HashSet<SocketAddr> = HashSet::new();
        let mut best: Vec<Entry> = Vec::with_capacity(K);
        let msg = KadMsg::Lookup(id, myself);
        let mut destinations = HashMap::new();

        let initial = ktable.lock().unwrap().closest_to(10, id);
        for ent in initial.into_iter() {
            destinations.insert(ent, &msg);
        }

        loop {
            // ask all destinations
            let responses = udp::send_with_responses(
                &sock,
                &destinations.iter().map(|(k,v)| (k.get_addr(), v)).collect(),
                3,
                Duration::from_millis(500),
                |_, m: &KadMsg| m.is_answer()
            ).expect("io error in id_lookup");

            // say that we have visited all destinations
            for d in destinations.keys() {
                visited.insert(d.get_addr());
            }

            // TODO: gör best till en ktable
            // 1) använd ktab som best
            // 2) uppdatera båda i tandemn
            // 3) spara ändringarna och applya dem i slutet till ktab

            // process all responses
            for (_, resp) in responses.iter() {
                if let KadMsg::Answer(ans) = resp {
                    let mut ktab = ktable.lock().unwrap();
                    for a in ans {
                        if ! visited.contains(&a.get_addr()) {
                            ktab.offer(*a);
                            insert_into_best(&mut best, *a, id);
                        }
                    }
                } else {
                    unreachable!();
                }
            }

            // remove dead connections from ktable
            {
                let mut ktab = ktable.lock().unwrap();
                for before in destinations.keys() {
                    if ! responses.contains_key(&before.get_addr()) {
                        ktab.delete_entry(*before);
                    }
                }
            }

            if has_looked_at_all(&visited, &best) {
                break;
            } else {
                // if 10 gå tillbaka till början igen
                destinations.clear();
                for b in best.iter() {
                    if ! visited.contains(&b.get_addr()) {
                        destinations.insert(*b, &msg);
                    }
                }
            }
        }
        tx.send(best).expect("caller killed its receive end");
    });
    rx
}

/// is every entry in `best` inside `visited`?
fn has_looked_at_all(visited: &HashSet<SocketAddr>, best: &Vec<Entry>) -> bool {
    for x in best.iter() {
        if ! visited.contains(&x.get_addr()) {
            return false;
        }
    }
    true
}

/// inserts `ele` into `best` if `ele` is closer to `id` than any other
/// entry in `best`. `ele` replaces another entry in `best` if `best`
/// is full.
/// does nothing if `ele` already is in `best`
fn insert_into_best(best: &mut Vec<Entry>, ele: Entry, id: Id) {
    assert!(best.capacity() > 0, "best vec size 0");
    if best.contains(&ele) {
        return;
    }
    if best.len() < best.capacity() {
        best.push(ele);
    } else {
        let mut worst = 0;
        let mut worst_dist = id.distance(&best[worst].get_id());
        for i in 1..best.len() {
            let temp = id.distance(&best[i].get_id());
            if temp > worst_dist {
                worst_dist = temp;
                worst = i;
            }
        }

        if ele.get_id().distance(&id) < worst_dist {
            best[worst] = ele;
        }
    }
}

/// queries all trackers and returns the first bootstrap node that is alive
/// returns Err(NetworkError::Timeout) if no tracker responded
pub fn find_bootstrapper(sock: &UdpSocket, room_id: Id, trackers: &Vec<SocketAddr>) -> Result<Option<(SocketAddr, Id)>> {
    let mut timedout = 0;
    'outer:
    for track in trackers.iter() {
        let sess = api::LookupSession::new(sock, *track, room_id);
        for b in sess {
            match b {
                Ok(adr) => {
                    if let Some(id) = is_alive(sock, adr)? {
                        return Ok(Some((adr, id)));
                    }
                },
                Err(NetworkError::Timeout) => {
                    info!("tracker {} timed out", track);
                    timedout += 1;
                    continue 'outer;
                },
                Err(e) => return Err(e),
            }
        }
    }
    if timedout == trackers.len() {
        Err(NetworkError::Timeout)
    } else {
        Ok(None)
    }
}

/// simply checks whether `adr` is an alive kademlia node and returns its id
pub fn is_alive(sock: &UdpSocket, adr: SocketAddr) -> Result<Option<Id>> {
    match udp::send_with_response(sock, &KadMsg::Ping, adr, 3, Duration::from_millis(500), |msg: &KadMsg| msg.is_pong()) {
        Ok(KadMsg::Pong(id)) => return Ok(Some(id)),
        Err(NetworkError::Timeout) => return Ok(None),
        Err(e) => return Err(e),
        Ok(_) => unreachable!(),
    }

}

/// handles one kademlia message
/// times out after `timeout`
pub fn handle_msg(sock: &UdpSocket, my_id: Id, timeout: Duration, ktable: Arc<Mutex<Ktable>>) -> Result<()> {
    match udp::recv_until_timeout(sock, timeout, |_,_| true) {
        Ok((sender, KadMsg::Ping)) => {
            debug!("{} pinged me!", sender);
            udp::send(sock, &KadMsg::Pong(my_id), sender)?;
            Ok(())
        },
        Ok((sender, KadMsg::Lookup(id, requester_entry))) => {
            // NOTE: sender is a temporary address
            let closest;
            {
                let mut ktab = ktable.lock().unwrap();
                closest = ktab.closest_to(K as u32, id);
                ktab.offer(requester_entry);
            }
            let clos_len = closest.len();
            udp::send(sock, &KadMsg::Answer(closest), sender)?;
            debug!("{} wanted to lookup {}, i answered with {}/{} nodes", sender, id, clos_len, K);
            Ok(())
        },
        Err(NetworkError::Timeout) => return Ok(()),
        Err(e) => return Err(e),
        _ => {
            warn!("received a KadMsg that i shouldn't have gotten");
            Ok(())
        },
    }
}
