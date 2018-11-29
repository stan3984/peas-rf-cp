use std::mem;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use log;

use net::fromnetmsg::FromNetMsg;
use net::tonetmsg::ToNetMsg;

const RECV_TIMEOUT: Duration = Duration::from_millis(50);

pub fn run(mut chan_in: Option<Receiver<ToNetMsg>>, mut chan_out: Option<Sender<FromNetMsg>>) {
    log::debug!("");

    event_loop(&mut chan_in, &mut chan_out);

    // drop and destroy the channels. this is probably not required to
    // be explicit, but I thought it would be nice.
    mem::drop(chan_in.take());
    mem::drop(chan_out.take());
}

fn event_loop(chan_in: &mut Option<Receiver<ToNetMsg>>, chan_out: &mut Option<Sender<FromNetMsg>>) {
    let mut finished: bool = false;

    while !finished {
        if let Some(ref mut c) = chan_in {
            match c.recv_timeout(RECV_TIMEOUT) {
                Err(RecvTimeoutError::Disconnected) => unreachable!(),
                Err(RecvTimeoutError::Timeout) => {}
                Ok(x) => match x {
                    ToNetMsg::Terminate => finished = true,
                    _ => {}
                },
            }
        }
    }
}
