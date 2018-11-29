use std::mem;
use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use log;
use node::util;

use net::fromnetmsg::FromNetMsg;
use net::netthread;
use net::tonetmsg::ToNetMsg;

#[derive(Debug, Clone)]
pub enum SendError {
    Disconnected(ToNetMsg),
    Dropped(ToNetMsg),
}

pub struct NetHandle {
    join_handle: JoinHandle<()>,
    channel_in: Option<Sender<ToNetMsg>>,
    channel_out: Option<Receiver<FromNetMsg>>,
}

impl NetHandle {
    pub fn new(with_output: bool) -> Self {
        log::debug!("Initializing new `NetHandle`");

        let (chan_out_send, chan_out_recv) = util::new_channel_in_option();

        let (chan_in_send, chan_in_recv) = if with_output {
            util::new_channel_in_option()
        } else {
            (None, None)
        };

        let jhandle = thread::spawn(move || {
            netthread::run(chan_in_recv, chan_out_send);
        });

        NetHandle {
            join_handle: jhandle,
            channel_in: chan_in_send,
            channel_out: chan_out_recv,
        }
    }

    /// Extracts a `JoinHandle` to the underlying thread.
    #[inline]
    pub fn join_handle(&self) -> &JoinHandle<()> {
        &self.join_handle
    }

    /// Closes and drops the input channel.
    ///
    /// Attempts to send messages through this `NetHandle` will fail
    /// when this function has executed.
    #[inline]
    pub fn close_input(&mut self) {
        if let Some(rx) = self.channel_in.take() {
            mem::drop(rx);
        }
    }

    /// Returns `true` if the input channel is open.
    #[inline]
    pub fn has_input(&self) -> bool {
        self.channel_in.is_some()
    }

    /// Closes and drops the output channel.
    ///
    /// The `NetHandle` will not be able to any send messages when
    /// this function has executed.
    #[inline]
    pub fn close_output(&mut self) {
        if let Some(tx) = self.channel_out.take() {
            mem::drop(tx);
        }
    }

    /// Returns `true` if the output channel is open.
    #[inline]
    pub fn has_output(&self) -> bool {
        self.channel_out.is_some()
    }

    /// Requests termination of the underlying thread.
    #[inline]
    pub fn terminate(&mut self) -> Result<(), SendError> {
        self.send_message(ToNetMsg::Terminate)
    }

    /// Updates the username.
    #[inline]
    pub fn set_username(&mut self, username: String) -> Result<(), SendError> {
        self.send_message(ToNetMsg::SetUsername(username))
    }

    /// Registers a new tracker to this client.
    #[inline]
    pub fn register_tracker(&mut self, socket: SocketAddr) -> Result<(), SendError> {
        self.send_message(ToNetMsg::RegisterTracker(socket))
    }

    /// Removes a registered tracker from this client.
    #[inline]
    pub fn unregister_tracker(&mut self, socket: SocketAddr) -> Result<(), SendError> {
        self.send_message(ToNetMsg::UnregisterTracker(socket))
    }

    fn send_message(&mut self, msg: ToNetMsg) -> Result<(), SendError> {
        match self.channel_in {
            Some(ref tx) => {
                match tx.send(msg) {
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
                        Err(SendError::Disconnected(val))
                    }
                }
            }
            None => {
                log::error!(
                    "Failed to send message `{:?}`: sender has been dropped",
                    msg
                );
                Err(SendError::Dropped(msg))
            }
        }
    }
}
