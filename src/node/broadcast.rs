
use node::ktable::{Ktable, Entry};
use std::net::SocketAddr;
use node::cache::Cache;
use network::udpmanager as UM;
use common::get_hash;
use common::id::Id;
use super::{Message,FromNetMsg};
use std::sync::{Mutex, Arc};
use std::sync::mpsc::Sender;
use common::timer::Timer;
use std::collections::HashMap;

const MAX_CONNECTIONS: u32 = 3;

/// handles everything that has to do with the broadcast network
pub struct BroadcastManager<'a> {
    connected: HashMap<SocketAddr, Id>,
    cache: Cache<u64>,
    active: Vec<(Msg, UM::SendHandle<()>)>,
    ktable: Arc<Mutex<Ktable>>,
    service: UM::ServiceHandle,
    udpman: &'a UM::Manager,
    chan_out: Sender<FromNetMsg>,
    my_id: Id,
    ting_timer: Timer,
    ting_cur: Option<Ting>,
}

struct Ting {
    dst: Entry,
    timer: Timer,
    sendh: UM::SendHandle<()>,
    sendh_done: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MsgPayload {
    IsAlive(Id),
    Msg(Message),
    Ting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Msg {
    hash: u64,
    sender_id: Id, // not the source, but the link that we received from
    payload: MsgPayload,
}

impl<'a> BroadcastManager<'a> {
    pub fn new(
        ktable: Arc<Mutex<Ktable>>,
        service: UM::ServiceHandle,
        udpman: &'a UM::Manager,
        chan_out: Sender<FromNetMsg>,
        my_id: Id
    ) -> Self {
        BroadcastManager{
            connected: HashMap::new(),
            cache: Cache::new(100),
            active: Vec::new(),
            ktable: ktable,
            service: service,
            udpman: udpman,
            chan_out: chan_out,
            my_id: my_id,
            ting_timer: Timer::from_millis(1000*10),
            ting_cur: None,
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

        // update and maybe start ting
        self.update_ting();

        // read and broadcast
        let mut count = 10;
        loop {
            if count == 0 {
                break;
            } else {
                count -= 1;
            }

            if let Some((Msg{hash, payload, sender_id}, sender, id)) = UM::service_get(&self.service) {
                UM::service_respond(&self.service, &(), id, sender).unwrap();

                if self.cache.insert(hash) {
                    let broadcast =
                        match payload {
                            MsgPayload::Msg(ref msg) => {
                                debug!("received msg: '{}'", msg.get_message());
                                let mut copy = msg.clone();
                                copy.is_myself = false;
                                self.chan_out.send(FromNetMsg::from_message(copy)).unwrap();
                                true
                            }
                            MsgPayload::IsAlive(alive_id) => {
                                if self.ting_cur.is_some() {
                                    if self.ting_cur.as_ref().unwrap().dst.get_id() == alive_id {
                                        self.ting_cur = None;
                                        self.ting_timer.reset();
                                        debug!("ting target could be reached");
                                    }
                                }
                                true
                            }
                            MsgPayload::Ting => {
                                debug!("responding to a ting");
                                let m = self.new_msg(MsgPayload::IsAlive(self.my_id));
                                self.broadcast_a_msg(m, None);
                                false
                            }
                        };

                    if broadcast {
                        // are we also connected to this fellow broadcaster?
                        if !self.connected.contains_key(&sender) {
                            self.connected.insert(sender, sender_id);
                            self.ktable.lock().unwrap().offer(Entry::new(sender, sender_id));
                        }

                        let my_id = self.my_id;
                        self.broadcast_a_msg(
                            Msg{
                                hash: hash,
                                payload: payload,
                                sender_id: my_id
                            },
                            Some(sender)
                        );
                    }
                }
            }
        }
    }

    fn remove_connection(&mut self, adr: &SocketAddr) {
        if self.connected.contains_key(&adr) {
            let id = self.connected.remove(&adr).unwrap();
            debug!("removed {} from connected", adr);
            self.ktable.lock().unwrap().delete_id(id);
        }
    }

    /// try to connect to peers if we are connected to too few
    fn connect_closest(&mut self) {
        let ktab = self.ktable.lock().unwrap();
        if self.connected.len() < MAX_CONNECTIONS as usize {
            let closest = ktab.get(MAX_CONNECTIONS);
            for c in closest {
                if !self.connected.contains_key(&c.get_addr()) {
                    debug!("connected to {}", c.get_addr());
                    self.connected.insert(c.get_addr(), c.get_id());
                }
            }
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
            self.connected.keys()
                .map(|e| *e)
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
        let m = self.from_message(msg);
        self.broadcast_a_msg(m, None);
    }

    fn update_ting(&mut self) {
        // check if ting should be started
        if self.ting_timer.expired(1.0) {
            assert!(self.ting_cur.is_none());
            let mran = self.ktable.lock().unwrap().random();

            if let Some(ran) = mran {
                self.ting_timer.disable();
                let m = Msg{hash: get_hash(), payload: MsgPayload::Ting, sender_id: self.my_id};
                let sh = UM::send(
                    &self.udpman,
                    &m,
                    vec![ran.get_addr()],
                    super::BROADCAST_SERVICE
                );
                self.cache.insert(m.hash);
                let mut t = Timer::from_millis(1000*3);
                t.disable();
                self.ting_cur = Some(Ting{dst: ran, sendh: sh, sendh_done: false, timer: t});
                debug!("sending a ting");

            } else {
                debug!("no one to ting");
                self.ting_timer.reset();
            }
        }
        // update ongoing ting
        // TODO: this is the ugliest thing i have ever seen
        if self.ting_cur.is_some() {
            if !self.ting_cur.as_ref().unwrap().sendh_done {
                self.ting_cur.as_mut().unwrap().sendh.update();

                if self.ting_cur.as_ref().unwrap().sendh.is_done() {
                    self.ting_cur.as_mut().unwrap().sendh_done = true;

                    if self.ting_cur.as_ref().unwrap().sendh.borrow_single_answer().is_some() {
                        self.ting_cur.as_mut().unwrap().timer.reset();
                    } else {
                        let i = self.ting_cur.as_ref().unwrap().dst;
                        self.ktable.lock().unwrap().delete_id(i.get_id());
                        self.ting_cur = None;
                        self.ting_timer.reset();
                    }
                }
            } else if self.ting_cur.as_ref().unwrap().timer.expired(1.0) {
                let i = self.ting_cur.as_ref().unwrap().dst;
                if !self.connected.contains_key(&i.get_addr()) {
                    debug!("creating an extra bridge to {} because a ting timed out", i.get_addr());
                    self.connected.insert(i.get_addr(), i.get_id());
                }
                self.ting_timer.reset();
                self.ting_cur = None;
            }
        }

    }
    fn new_msg(&self, pay: MsgPayload) -> Msg {
        Msg{hash: get_hash(), payload: pay, sender_id: self.my_id}
    }
    fn from_message(&self, msg: Message) -> Msg {
        self.new_msg(MsgPayload::Msg(msg))
    }
}

