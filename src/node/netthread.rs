use std::mem;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use std::sync::{Arc, Mutex};

use super::*;
use network::udp;
use common::id::Id;

const RECV_TIMEOUT: Duration = Duration::from_millis(50);

pub fn run(chan_in: Receiver<ToNetMsg>,
           chan_out: Sender<FromNetMsg>,
           with_output: bool, // TODO: är antagligen bättre att ha chan_out som en option
           user_id: Id,
           user_name: String,
           room_id: Id,
           trackers: Vec<SocketAddr>
) {

    let kad_sock = udp::open_any().unwrap();
    let my_id = Id::new_random();
    let myself = ktable::Entry::new(kad_sock.local_addr().unwrap(), my_id);
    let ktab = kademlia::create_ktable(my_id);
    info!("my id is {}", my_id);
    let mut looking = None;
    let mut first_lookup = true;

    // TODO: check if all trackers are dead
    let mut tracker_timer = Timer::from_millis(0);
    let mut lookup_timer = Timer::from_millis(1000*60);
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
                    tracker_timer = Timer::new(ttl);
                },
                Err(NetworkError::Timeout) => {
                    warn!("tracker on update is not responding");
                    tracker_timer.disable();
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
            }
        }

        // lookup from bootstrapper is done
        if first_lookup && looking.is_none() {
            first_lookup = false;
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
        //read from chan_in
        // chan_in.try_recv();
        chan_out.send(FromNetMsg::NewMsg(Message::new("hej".to_string(), Id::from_u64(2), "kalle".to_string(), false))).unwrap();
    }

}

