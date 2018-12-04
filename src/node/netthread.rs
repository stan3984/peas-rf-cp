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
    let boot_node = kademlia::find_bootstrapper(&kad_sock, room_id, &trackers).unwrap();
    if let Some((adr, id)) = boot_node {
        info!("found {:?} to bootstrap to", adr);
        ktab.lock().unwrap().offer(ktable::Entry::new(adr, id));
        looking = Some(kademlia::id_lookup(udp::open_any().unwrap(), my_id, myself, ktab.clone()));
    } else {
        info!("you are the first one to connect to this room");
        first_lookup = false;
    }

    loop {
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
        //handle one tcp? or maybe in own thread?
        //read from chan_in
        // chan_in.try_recv();
        chan_out.send(FromNetMsg::NewMsg(Message::new("hej".to_string(), Id::from_u64(2), "kalle".to_string(), false))).unwrap();
    }

}

