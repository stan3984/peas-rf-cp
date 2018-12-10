
use node::ktable::{Ktable, Entry};
use std::net::SocketAddr;
use node::cache::Cache;
use network::udpmanager as UM;
use common::get_hash;
use common::id::Id;
use super::{Message,FromNetMsg};
use std::sync::{Mutex, Arc};
use std::sync::mpsc::Sender;

const MAX_CONNECTIONS: u32 = 3;

/// handles everything that has to do with the broadcast network
pub struct BroadcastManager<'a> {
    connected: Vec<Entry>,
    cache: Cache<u64>,
    active: Vec<(Msg, UM::SendHandle<()>)>,
    ktable: Arc<Mutex<Ktable>>,
    service: UM::ServiceHandle,
    udpman: &'a UM::Manager,
    chan_out: Sender<FromNetMsg>,
}

impl<'a> BroadcastManager<'a> {
    pub fn new(
        ktable: Arc<Mutex<Ktable>>,
        service: UM::ServiceHandle,
        udpman: &'a UM::Manager,
        chan_out: Sender<FromNetMsg>
    ) -> Self {
        BroadcastManager{
            connected: Vec::new(),
            cache: Cache::new(100),
            active: Vec::new(),
            ktable: ktable,
            service: service,
            udpman: udpman,
            chan_out: chan_out,
        }
    }

    pub fn update(&mut self) {

        let mut resend: Vec<Msg> = Vec::new();

        // update and remove done active broadcasts
        for i in (0..self.active.len()).rev() {
            self.active[i].1.update();

            if self.active[i].1.is_done() {
                let (m, sh) = self.active.remove(i);
                let mut want_to_resend = false;

                for a in sh.iter() {
                    if sh.is_dead(a) {
                        self.remove_connection(&a);
                        want_to_resend = true;
                    }
                }

                if want_to_resend {
                    resend.push(m);
                }
            }
        }

        // connect to more
        self.connect_closest();

        // TODO: optimera
        // resend stuff where atleast one connection didn't respond
        for r in resend.into_iter() {
            debug!("resending something");
            self.broadcast_a_msg(r, None);
        }

        // read and broadcast
        let mut count = 10;
        loop {
            if count == 0 {
                break;
            } else {
                count -= 1;
            }

            if let Some((Msg{hash, payload}, sender, id)) = UM::service_get(&self.service) {
                UM::service_respond(&self.service, &(), id, sender).unwrap();
                if self.cache.insert(hash) {
                    self.broadcast_a_msg(Msg{hash: hash, payload: payload.clone()}, Some(sender));
                    match payload {
                        MsgPayload::Msg(mut msg) => {
                            debug!("received msg: '{}'", msg.get_message());
                            msg.is_myself = false;
                            self.chan_out.send(FromNetMsg::from_message(msg)).unwrap();
                        }
                        MsgPayload::IsAlive(alive_id) => {
                            // TODO: 
                        }
                    }
                }
            }
        }
    }

    fn remove_connection(&mut self, adr: &SocketAddr) {
        for i in (0..self.connected.len()).rev() {
            if self.connected[i].get_addr() == *adr {
                debug!("removed {} from connected", self.connected[i].get_addr());
                self.ktable.lock().unwrap().delete_id(self.connected[i].get_id());
                self.connected.remove(i);
                break;
            }
        }
    }

    /// try to connect to peers if we are connected to too few
    fn connect_closest(&mut self) {
        let ktab = self.ktable.lock().unwrap();
        if self.connected.len() < MAX_CONNECTIONS as usize {
            let closest = ktab.get(MAX_CONNECTIONS);
            for c in closest {
                if !already_connected(&self.connected, c) {
                    debug!("connected to {}", c.get_addr());
                    self.connected.push(c);
                }
            }
        }
        fn already_connected(streams: &Vec<Entry>, e: Entry) -> bool {
            for s in streams {
                if s.get_id() == e.get_id() {
                    return true
                }
            }
            false
        }
    }

    /// broadcast `msg` to all other nodes
    fn broadcast_a_msg(&mut self, msg: Msg, ban: Option<SocketAddr>) {
        if self.connected.is_empty() {
            warn!("no one to send to, dropping the message");
            self.chan_out.send(FromNetMsg::NotSent).unwrap();
            return;
        }

        self.cache.insert(msg.hash);

        let targets: Vec<SocketAddr> =
            self.connected.iter()
                .map(|e| e.get_addr())
                .filter(|a| ban.map_or(true, |b| b != *a))
                .collect();

        // no one to send to
        if targets.is_empty() {
            return;
        }

        let sh = UM::send(
            &self.udpman,
            &msg,
            targets,
            super::BROADCAST_SERVICE,
        );

        self.active.push((msg, sh));
    }

    /// broadcasts a new message to everyone else
    pub fn broadcast(&mut self, msg: Message) {
        self.broadcast_a_msg(Msg::from_message(msg), None);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    hash: u64,
    payload: MsgPayload,
}

impl Msg {
    pub fn new(pay: MsgPayload) -> Self {
        Msg{hash: get_hash(), payload: pay}
    }
    pub fn from_message(msg: Message) -> Self {
        Msg::new(MsgPayload::Msg(msg))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MsgPayload {
    IsAlive(Id),
    Msg(Message),
}
