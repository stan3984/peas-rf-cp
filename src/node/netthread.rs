use std::mem;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

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

    let track_sock = udp::open_any().unwrap();
    let kadem_sock = udp::open_any().unwrap();
    let mut ktab = ktable::Ktable::new(3);
    let my_id = Id::new_random();

    let boot_node = kademlia::find_bootstrapper(&track_sock, room_id, &trackers).unwrap();
    if let Some((adr, id)) = boot_node {
        info!("found {:?} to bootstrap to", adr);
        // add adr and id to ktable and run lookup on my_id
    } else {
        info!("you are the first one to connect to this room");
    }

    loop {
        //handle one kademlia message, or timeout
        //handle one tcp? or maybe in own thread?
        //read from chan_in
    }

}
