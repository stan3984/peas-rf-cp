use std::mem;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use super::*;
use network::{tcp, udp};
use network::NetworkError;
use std::net::TcpStream;
use common::id::Id;
use tracker::api;
use common::timer::Timer;
use node::ktable::{Entry,Ktable};
use node::cache::Cache;

const RECV_TIMEOUT: Duration = Duration::from_millis(50);
const MAX_CONNECTIONS: u32 = 3;

pub fn run(chan_in: Receiver<ToNetMsg>,
           chan_out: Sender<FromNetMsg>,
           with_output: bool, // TODO: är antagligen bättre att ha chan_out som en option
           room_id: Id,
           trackers: Vec<SocketAddr>
) {

    let kad_sock = udp::open_any().unwrap();
    let tcp_sock = tcp::open_any().unwrap();
    let mut tcp_streams: Vec<TcpStream> = Vec::new();
    let mut cache: Cache<u64> = Cache::new(100);
    let my_id = Id::new_random();
    let myself = ktable::Entry::new(kad_sock.local_addr().unwrap(), my_id);
    let ktab = kademlia::create_ktable(my_id);

    info!("my id is {}, and my address is {}", my_id, kad_sock.local_addr().unwrap());

    let mut looking = None;
    let mut first_lookup = true;

    let mut tracker_timer = Timer::from_millis(0);
    let mut lookup_timer = Timer::from_millis(1000*60);

    // find a bootstrapper
    let boot_node = kademlia::find_bootstrapper(&kad_sock, room_id, &trackers).unwrap();
    if let Some((adr, id)) = boot_node {
        info!("found {:?} to bootstrap to", adr);
        ktab.lock().unwrap().offer(ktable::Entry::new(adr, id));
        looking = Some(kademlia::id_lookup(udp::open_any().unwrap(), my_id, myself, ktab.clone()));
    } else {
        info!("you are the first one to connect to this room");
        // TODO: periodically check the trackers if something just goofed
        first_lookup = false;
    }

    'main:
    loop {
        // check if we need to update ourself in the tracker
        // TODO: we are only assuming we have one tracker
        if tracker_timer.expired(0.95) {
            debug!("we are now updating ourselves in a (the) tracker");
            match api::update(&udp::open_any().unwrap(),
                              room_id,
                              kad_sock.local_addr().unwrap(),
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

        //handle one kademlia message, or timeout
        kademlia::handle_msg(&kad_sock, my_id, Duration::from_millis(300), ktab.clone()).expect("io error from handle_msg");

        // check if a lookup completed
        if looking.is_some() {
            if looking.as_mut().unwrap().try_recv().is_ok() {
                looking = None;
                debug!("a lookup finished");
            }
        }

        // lookup from bootstrapper is done
        if first_lookup && looking.is_none() {
            first_lookup = false;
            debug!("the bootstrap lookup finished");
        }

        // run an id_lookup on a random id to update our and all others ktables
        if looking.is_none() && lookup_timer.expired(1.0) {
            debug!("a random id lookup started");
            looking = Some(kademlia::id_lookup(udp::open_any().unwrap(),
                                               Id::new_random(),
                                               myself,
                                               ktab.clone()));
            lookup_timer.reset();
        }

        //handle one tcp? or maybe in own thread?
        match tcp::accept(&tcp_sock) {
            Ok(stream) => {
                debug!("added {} to tcp list", stream.peer_addr().unwrap());
                tcp_streams.push(stream);
            },
            Err(NetworkError::Timeout) => (),
            Err(ioerror) => panic!(ioerror),
        }

        read_and_broadcast(&mut tcp_streams,
                           &mut ktab.lock().unwrap(),
                           &mut cache,
                           &chan_out);
        connect_tcps(&mut tcp_streams, &mut ktab.lock().unwrap());

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
                broadcast_except(&mut tcp_streams, None, &tosend);
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                warn!("I don't have a master any more, terminating...");
                break 'main;
            }
        }
    }

    // TODO: gracefully tell everyone else that i am quitting
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

fn broadcast_except(streams: &mut Vec<TcpStream>, banned: Option<SocketAddr>, msg: &TcpBroadcast) {
    for s in streams {
        if banned.map_or(true, |b| b != s.peer_addr().unwrap()) {
            tcp::send(s, msg).map_err(|e| warn!("on tcp::send {}", e)).unwrap_or(0);
        }
    }
}

/// try to connect to peers if we are connected to too few
fn connect_tcps(streams: &mut Vec<TcpStream>, ktab: &mut Ktable) {
    if streams.len() < MAX_CONNECTIONS as usize {
        let closest = ktab.get(MAX_CONNECTIONS);
        for c in closest {
            if !already_connected(streams, c) {
                if let Ok(s) = tcp::connect(c.get_addr()) {
                    streams.push(s);
                    debug!("connected to {}", c.get_addr());
                } else {
                    ktab.delete_entry(c);
                    debug!("couldn't connect to {}, removing it", c.get_addr());
                }
            }
        }
    }
    fn already_connected(streams: &Vec<TcpStream>, e: Entry) -> bool {
        for s in streams {
            if s.local_addr().unwrap() == e.get_addr() {
                return true
            }
        }
        false
    }
}
