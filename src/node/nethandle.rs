use std::mem;
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
    my_name: String,
    my_id: Id,
}

impl NetHandle {
    pub fn new(with_output: bool,
               user_id: Id,
               user_name: String,
               room_id: Id,
               trackers: Vec<SocketAddr>
    ) -> Self {
        log::debug!("Initializing new `NetHandle`");

        let (chan_out_send, chan_out_recv) = channel();
        let (chan_in_send, chan_in_recv) = channel();

        let jhandle = thread::spawn(move || {
            netthread::run(chan_in_recv, chan_out_send, with_output, room_id, trackers);
        });

        NetHandle {
            join_handle: jhandle,
            channel_in: chan_in_send,
            channel_out: chan_out_recv,
            my_name: user_name,
            my_id: user_id,
        }
    }

    // /// Extracts a `JoinHandle` to the underlying thread.
    // #[inline]
    // pub fn join_handle(&self) -> &JoinHandle<()> {
    //     &self.join_handle
    // }

    // /// Closes and drops the input channel.
    // ///
    // /// Attempts to send messages through this `NetHandle` will fail
    // /// when this function has executed.
    // #[inline]
    // pub fn close_input(&mut self) {
    //     if let Some(rx) = self.channel_in.take() {
    //         mem::drop(rx);
    //     }
    // }

    // /// Returns `true` if the input channel is open.
    // #[inline]
    // pub fn has_input(&self) -> bool {
    //     self.channel_in.is_some()
    // }

    // /// Closes and drops the output channel.
    // ///
    // /// The `NetHandle` will not be able to any send messages when
    // /// this function has executed.
    // #[inline]
    // pub fn close_output(&mut self) {
    //     if let Some(tx) = self.channel_out.take() {
    //         mem::drop(tx);
    //     }
    // }

    // /// Returns `true` if the output channel is open.
    // #[inline]
    // pub fn has_output(&self) -> bool {
    //     self.channel_out.is_some()
    // }

    // /// Requests termination of the underlying thread.
    // #[inline]
    // pub fn terminate(&mut self) -> Result<(), SendError> {
    //     self.send_message(ToNetMsg::Terminate)
    // }

    // /// Updates the username.
    // #[inline]
    // pub fn set_username(&mut self, id: Id, username: String) -> Result<(), SendError> {
    //     self.send_message(ToNetMsg::SetUsername(id, username))
    // }

    // /// Registers a new tracker to this client.
    // #[inline]
    // pub fn register_tracker(&mut self, socket: SocketAddr) -> Result<(), SendError> {
    //     self.send_message(ToNetMsg::RegisterTracker(socket))
    // }

    // /// Removes a registered tracker from this client.
    // #[inline]
    // pub fn unregister_tracker(&mut self, socket: SocketAddr) -> Result<(), SendError> {
    //     self.send_message(ToNetMsg::UnregisterTracker(socket))
    // }

    /// tries to read something from the nethandle if it has something to say.
    /// returns Ok(Some(msg)) if it had something to say
    /// returns Ok(None) if it didn't
    /// Err(SendError::Disconnected) if the nethandle died
    pub fn read(&mut self) -> Result<Option<FromNetMsg>, SendError> {
        match self.channel_out.try_recv() {
            Ok(ok) => Ok(Some(ok)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(SendError::Disconnected),
        }
    }

    /// sends `msg` to all other connected nodes.
    /// returns the message struct that was sent
    pub fn send_message(&mut self, msg: String) -> Result<Message, SendError> {
        let m = Message::new(msg, self.my_id, self.my_name.clone(), true);
        self.send_to_net(ToNetMsg::NewMsg(m.clone()))?;
        Ok(m)
    }

    fn send_to_net(&mut self, msg: ToNetMsg) -> Result<(), SendError> {
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
