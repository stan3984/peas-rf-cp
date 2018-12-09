
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
use network::udpmanager as UM;

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

/// represents an ongoing id lookup
pub struct IdLookup<'a> {
    udpman: &'a UM::Manager,
    visited: HashSet<SocketAddr>,
    best: Ktable,
    ktable: Arc<Mutex<Ktable>>,
    msg: KadMsg,
    cur: Option<UM::SendHandle<KadMsg>>,
    map: HashMap<SocketAddr, Id>,
}

impl<'a> IdLookup<'a> {
    /// initializes and starts an id lookup on `lookup_id`
    /// It will update `ktable` continously
    pub fn new(udpman: &'a UM::Manager, lookup_id: Id, myself: Entry, ktable: Arc<Mutex<Ktable>>) -> Self {
        let msg = KadMsg::Lookup(lookup_id, myself);

        let initial: HashMap<SocketAddr, Id> = ktable.lock().unwrap()
            .closest_to(2*K as u32, lookup_id)
            .into_iter()
            .map(|e| (e.get_addr(), e.get_id()))
            .collect();

        let sendh = UM::send(
            udpman,
            &msg,
            initial.keys().map(|k| *k).collect(),
            super::KAD_SERVICE
        );

        IdLookup {
            udpman: udpman,
            visited: HashSet::new(),
            best: Ktable::new(K as u32, lookup_id),
            ktable: ktable,
            msg: msg,
            cur: Some(sendh),
            map: initial,
        }
    }

    /// is the lookup done?
    pub fn is_done(&self) -> bool {
        self.cur.is_none()
    }

    /// call this over and over until it is done
    pub fn update(&mut self) {
        self.tick(false);
    }

    /// block the thread and wait for the lookup to finish
    pub fn update_wait(&mut self) {
        while !self.is_done() {
            self.tick(true);
        }
    }

    fn tick(&mut self, block: bool) {
        if self.is_done() {
            return
        }

        if block {
            self.cur.as_mut().unwrap().update_wait();
        } else {
            self.cur.as_mut().unwrap().update();
        }

        if !self.cur.as_ref().unwrap().is_done() {
            return;
        }

        // say that we have visited all destinations
        for c in self.cur.as_mut().unwrap().iter() {
            self.visited.insert(*c);
        }

        // process all responses
        {
            let mut ktab = self.ktable.lock().unwrap();
            for c in self.cur.as_ref().unwrap().iter() {
                // remove dead
                if self.cur.as_ref().unwrap().is_dead(c) {
                    let before = self.map.get(c).unwrap();
                    ktab.delete_id(*before);
                    self.best.delete_id(*before);
                } else {
                    // is alive, add it
                    if let KadMsg::Answer(ans) = self.cur.as_ref().unwrap().borrow_answer(c) {
                        for a in ans {
                            if !self.visited.contains(&a.get_addr()) {
                                ktab.offer(*a);
                                self.best.offer(*a);
                            }
                        }
                    } else {
                        error!("id lookup got something that was not KadMsg::Answer");
                    }
                }
            }
        }

        // start a new round
        let all = self.best.get(u32::max_value());
        let kbest = &all[..std::cmp::min(all.len(), K+1)];
        if self.has_looked_at_all(kbest) {
            self.cur = None;
        } else {
            self.map.clear();
            for b in all.iter() {
                if self.map.len() >= K {
                    break;
                }
                if !self.visited.contains(&b.get_addr()) {
                    self.map.insert(b.get_addr(), b.get_id());
                }
            }
            self.cur = Some(UM::send(
                self.udpman,
                &self.msg,
                self.map.keys().map(|k| *k).collect(),
                super::KAD_SERVICE
            ));
        }
    }

    /// is every entry in `best` inside `visited`?
    fn has_looked_at_all(&self, best: &[Entry]) -> bool {
        for x in best.iter() {
            if !self.visited.contains(&x.get_addr()) {
                return false;
            }
        }
        true
    }

    /// turns this lookup into the actual answer
    pub fn into_answer(self) -> Vec<Entry> {
        self.best.get(K as u32)
    }
}

/// creates a ktable in a mutex for cross thread use
pub fn create_ktable(my_id: Id) -> Arc<Mutex<Ktable>> {
    Arc::new(Mutex::new(Ktable::new(K as u32, my_id)))
}

/// queries all trackers and returns the first bootstrap node that is alive
/// returns Err(NetworkError::Timeout) if no tracker responded
// TODO: update tracker to remove `sock`
pub fn find_bootstrapper(udpman: &UM::Manager, sock: &UdpSocket, room_id: Id, trackers: &Vec<SocketAddr>) -> Result<Option<(SocketAddr, Id)>> {
    let mut timedout = 0;
    'outer:
    for track in trackers.iter() {
        let sess = api::LookupSession::new(sock, *track, room_id);
        for b in sess {
            match b {
                Ok(adr) => {
                    if let Some(id) = is_alive(udpman, adr) {
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
pub fn is_alive(udpman: &UM::Manager, adr: SocketAddr) -> Option<Id> {

    let mut sendh =
        UM::send(
            udpman,
            &KadMsg::Ping,
            vec![adr],
            super::KAD_SERVICE
        );

    sendh.update_wait();

    match sendh.get_single_answer() {
        Some(KadMsg::Pong(id)) => Some(id),
        Some(_) => {warn!("answer was not Pong"); None},
        None => None,
    }
}

/// handles many kademlia messages
pub fn handle_msg(servh: &UM::ServiceHandle, my_id: Id, ktable: Arc<Mutex<Ktable>>) -> Result<()> {
    let mut counter = 10;
    loop {
        if counter == 0 {
            break;
        } else {
            counter -= 1;
        }
        match UM::service_get(servh) {
            None => break,
            Some((KadMsg::Ping, sender, id)) => {
                debug!("{} pinged me!", sender);
                UM::service_respond(
                    servh,
                    &KadMsg::Pong(my_id),
                    id,
                    sender
                )?;
            },
            Some((KadMsg::Lookup(look_id, requester_entry), sender, id)) => {
                // NOTE: sender is a temporary address
                let mut closest;
                {
                    let mut ktab = ktable.lock().unwrap();
                    closest = ktab.closest_to(K as u32 + 1, look_id);
                    ktab.offer(requester_entry);
                }
                closest.retain(|e| e.get_id() != requester_entry.get_id());
                if closest.len() > K {
                    closest.pop();
                }
                let clos_len = closest.len();
                UM::service_respond(
                    servh,
                    &KadMsg::Answer(closest),
                    id,
                    sender
                )?;
                debug!("{} wanted to lookup {}, i answered with {}/{} nodes", sender, look_id, clos_len, K);
            },
            Some(_) => {
                warn!("someone sent weird KadMsg to kad_service");
            }
        }
    }
    Ok(())
}
