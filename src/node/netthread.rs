use std::mem;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::thread;

use super::*;
use network::{tcp, udp};
use network::NetworkError;
use network::udpmanager as UM;
use std::net::TcpStream;
use common::id::Id;
use tracker::api;
use common::timer::Timer;
use node::ktable::{Entry,Ktable};
use node::cache::Cache;

// const RECV_TIMEOUT: Duration = Duration::from_millis(50);
const MAX_CONNECTIONS: u32 = 3;
const THREAD_SLEEP: Duration = Duration::from_millis(40);

pub fn run(chan_in: Receiver<ToNetMsg>,
           chan_out: Sender<FromNetMsg>,
           with_output: bool, // TODO: är antagligen bättre att ha chan_out som en option
           room_id: Id,
           trackers: Vec<SocketAddr>
) {

    let kad_sock = udp::open_any().unwrap();
    let local_addr = kad_sock.local_addr().unwrap();
    let track_sock = udp::open_any().unwrap();
    let my_id = Id::new_random();
    let myself = ktable::Entry::new(local_addr, my_id);
    let ktab = kademlia::create_ktable(my_id);

    let udpman = UM::Manager::start(kad_sock);
    let kad_service = udpman.register_service(KAD_SERVICE);
    let broad_service = udpman.register_service(BROADCAST_SERVICE);

    info!("my id is {}, and my address is {}", my_id, local_addr);

    {
        // find a bootstrapper
        let boot_node = kademlia::find_bootstrapper(&udpman, &track_sock, room_id, &trackers).unwrap();
        if let Some((adr, id)) = boot_node {
            info!("found {:?} to bootstrap to", adr);
            ktab.lock().unwrap().offer(ktable::Entry::new(adr, id));
            kademlia::IdLookup::new(
                &udpman,
                my_id,
                myself,
                ktab.clone()
            ).update_wait();
        } else {
            info!("you are the first one to connect to this room");
            // TODO: periodically check the trackers if something just goofed
        }

        let mut cache: Cache<u64> = Cache::new(100);
        let mut connected: Vec<Entry> = Vec::new();
        let mut looking: Option<kademlia::IdLookup> = None;

        let mut tracker_timer = Timer::new_expired();
        let mut lookup_timer = Timer::from_millis(1000*20);

        'main:
        loop {
            // check if we need to update ourself in the tracker
            // TODO: we are only assuming we have one tracker
            if tracker_timer.expired(0.95) {
                debug!("we are now updating ourselves in a (the) tracker");
                udp::clear(&track_sock).unwrap();
                // can block for potentially long time
                match api::update(&track_sock,
                                  room_id,
                                  local_addr,
                                  trackers[0])
                {
                    Ok(ttl) => {
                        debug!("we are updated for {} seconds", ttl.as_secs());
                        tracker_timer.reset_with(ttl);
                    },
                    Err(NetworkError::Timeout) => {
                        let again = 10;
                        warn!("tracker on update is not responding, trying again in {}s", again);
                        tracker_timer.reset_with(Duration::from_secs(again));
                    },
                    Err(e) => {
                        error!("tracker update severe error {:?}", e);
                        tracker_timer.disable();
                    }
                }
            }

            //handle kademlia messages
            kademlia::handle_msg(&kad_service, my_id, ktab.clone()).expect("io error from handle_msg");

            // update an ongoing id lookup
            if looking.is_some() {
                looking.as_mut().unwrap().update();
                if looking.as_ref().unwrap().is_done() {
                    debug!("id lookup finished");
                    looking = None;
                    lookup_timer.reset();
                }
            }

            //run an id_lookup on a random id to update our and all others ktables
            if looking.is_none() && lookup_timer.expired(1.0) {
                debug!("a random id lookup started");
                looking = Some(kademlia::IdLookup::new(
                    &udpman,
                    Id::new_random(),
                    myself,
                    ktab.clone()
                ));
            }

            //handle one tcp? or maybe in own thread?

            //check if someone wants to say something
            // TODO: loop this to read more stuff?
            match chan_in.try_recv() {
                Ok(ToNetMsg::Terminate) => {
                    info!("netthread is terminating as per request...");
                    break 'main;
                }
                Ok(ToNetMsg::NewMsg(msg)) => {
                    debug!("'{}' is broadcasting '{}'", msg.get_sender_name(), msg.get_message());
                    let tosend = TcpBroadcast::from_message(msg);
                    cache.insert(tosend.hash);
                    // broadcast_except(&mut tcp_streams, None, &tosend);
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    error!("I don't have a master any more, terminating...");
                    break 'main;
                }
            }

            thread::sleep(THREAD_SLEEP);
        }
    }
    // TODO: gracefully tell everyone else that i am quitting
    udpman.terminate();
    info!("netthread terminated");
}

fn read_and_broadcast(streams: &mut Vec<TcpStream>, ktab: &mut Ktable, cache: &mut Cache<u64>, chan_out: &Sender<FromNetMsg>) {
    let timer = Timer::from_millis(10);
    for i in (0..streams.len()).rev() {
        loop {
            match tcp::recv_once::<TcpBroadcast>(&mut streams[i]) {
                Ok(msg) => {
                    if !cache.contains(&msg.hash) {
                        cache.insert(msg.hash);
                        let ban = streams[i].peer_addr().unwrap();
                        broadcast_except(streams, Some(ban), &msg);
                        if let TcpPayload::Msg(m) = msg.payload {
                            debug!("received '{}' from '{}'", m.get_message(), m.get_sender_name());
                            chan_out.send(FromNetMsg::from_message(m)).unwrap();
                        }
                    }
                },
                Err(NetworkError::Timeout) => break,
                Err(NetworkError::NoMessage) => (),
                Err(ioerror) => warn!("on tcp::recv_once {}", ioerror),
            }
        }
    }
}

/// send
fn broadcast_except(streams: &mut Vec<TcpStream>, banned: Option<SocketAddr>, msg: &TcpBroadcast) {
    for s in streams {
        if banned.map_or(true, |b| b != s.peer_addr().unwrap()) {
            tcp::send(s, msg).map_err(|e| warn!("on tcp::send {}", e)).unwrap_or(0);
        }
    }
}

/// try to connect to peers if we are connected to too few
fn connect_closest(streams: &mut Vec<Entry>, ktab: &mut Ktable) {
    if streams.len() < MAX_CONNECTIONS as usize {
        let closest = ktab.get(MAX_CONNECTIONS);
        for c in closest {
            if !already_connected(streams, c) {
                streams.push(c);
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
