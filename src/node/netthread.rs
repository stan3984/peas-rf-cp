use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::time::Duration;
use std::thread;

use super::*;
use network::NetworkError;
use network::udpmanager as UM;
use network::udp;
use common::id::Id;
use tracker::api;
use common::timer::Timer;
use node::broadcast::BroadcastManager;

// const RECV_TIMEOUT: Duration = Duration::from_millis(50);
const THREAD_SLEEP: Duration = Duration::from_millis(40);

pub fn run(chan_in: Receiver<ToNetMsg>,
           chan_out: Sender<FromNetMsg>,
           user_id: Id,
           user_name: String,
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
        let boot_node = kademlia::find_bootstrapper(&udpman, &track_sock, room_id, &trackers).expect("no tracker responded");
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

        // ongoing id lookup
        let mut looking: Option<kademlia::IdLookup> = None;
        let mut broadcast_man = BroadcastManager::new(ktab.clone(), broad_service, &udpman, chan_out.clone(), my_id);

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
                        let again = 60;
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

            //handle broadcasts
            broadcast_man.update();

            //check if someone wants to say something
            // TODO: loop this to read more stuff?
            match chan_in.try_recv() {
                Ok(ToNetMsg::Terminate) => {
                    info!("netthread is terminating as per request...");
                    break 'main;
                }
                Ok(ToNetMsg::NewMsg(ref msg)) if msg.len() > 100 => {
                    warn!("message longer than 100 characters, didn't send it");
                    chan_out.send(FromNetMsg::NotSent).unwrap();
                }
                Ok(ToNetMsg::NewMsg(msg)) => {
                    // debug!("'{}' is broadcasting '{}'", msg.get_sender_name(), msg.get_message());
                    let m = Message::new(msg, user_id, user_name.clone(), true);
                    chan_out.send(FromNetMsg::NewMsg(m.clone())).unwrap();
                    broadcast_man.broadcast(m);
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

