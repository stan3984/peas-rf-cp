use std::net::SocketAddr;
use std::sync::mpsc::{TryRecvError, Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use log;

use super::*;

#[derive(Debug, Clone)]
pub enum SendError {
    Disconnected,
    // Dropped,
}

pub struct NetHandle {
    join_handle: JoinHandle<()>,
    channel_in: Sender<ToNetMsg>,
    channel_out: Receiver<FromNetMsg>,
}

impl NetHandle {
    pub fn new(
        user_id: Id,
        user_name: String,
        room_id: Id,
        trackers: Vec<SocketAddr>
    ) -> Self {
        log::debug!("Initializing new `NetHandle`");

        let (chan_out_send, chan_out_recv) = channel();
        let (chan_in_send, chan_in_recv) = channel();

        let jhandle = thread::spawn(move || {
            netthread::run(
                chan_in_recv,
                chan_out_send,
                user_id,
                user_name,
                room_id,
                trackers);
        });

        NetHandle {
            join_handle: jhandle,
            channel_in: chan_in_send,
            channel_out: chan_out_recv,
        }
    }

    /// tries to read something from the nethandle if it has something to say.
    /// returns Ok(Some(msg)) if it had something to say
    /// returns Ok(None) if it didn't
    /// Err(SendError::Disconnected) if the nethandle died
    pub fn read(&self) -> Result<Option<FromNetMsg>, SendError> {
        match self.channel_out.try_recv() {
            Ok(ok) => Ok(Some(ok)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(SendError::Disconnected),
        }
    }

    /// sends `msg` to all other connected nodes.
    /// returns the message struct that was sent
    pub fn send_message(&self, msg: String) -> Result<(), SendError> {
        self.send_to_net(ToNetMsg::NewMsg(msg))?;
        Ok(())
    }

    fn send_to_net(&self, msg: ToNetMsg) -> Result<(), SendError> {
        match self.channel_in.send(msg) {
            Ok(x) => Ok(x),
            Err(se) => {
                // this only happens when the receiving end has
                // disconnected in which case data will never be
                // received
                let val = se.0;
                log::error!(
                    "Failed to send message `{:?}`: receiver has been disconnected",
                    val
                );
                Err(SendError::Disconnected)
            }
        }
    }
}
