use std::mem;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use super::*;
use network::udp;
use network::NetworkError;
use common::id::Id;
use tracker::api;
use common::timer::Timer;

const RECV_TIMEOUT: Duration = Duration::from_millis(50);

pub fn run(chan_in: Receiver<ToNetMsg>,
           chan_out: Sender<FromNetMsg>,
           with_output: bool, // TODO: är antagligen bättre att ha chan_out som en option
           room_id: Id,
           trackers: Vec<SocketAddr>
) {

    let kad_sock = udp::open_any().unwrap();
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
            // setup initial tcp
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

        //check if someone wants to say something
        // TODO: loop this to read more stuff?
        match chan_in.try_recv() {
            Ok(ToNetMsg::Terminate) => {
                info!("netthread is terminating as per request...");
                break 'main;
            }
            Ok(ToNetMsg::NewMsg(msg)) => (), //// TODO: send to all others
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                warn!("I don't have a master any more, terminating...");
                break 'main;
            }
        }
        // chan_out.send(FromNetMsg::NewMsg(Message::new("hej".to_string(), Id::from_u64(2), "kalle".to_string(), false))).unwrap();
    }

    // TODO: gracefully tell everyone else that i am quitting

}
