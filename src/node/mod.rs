mod ktable;
mod nethandle;

use std::net::SocketAddr;
use common::id::Id;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum FromNetMsg {
    Error(Option<String>),
    NewMsg(Message),
}

#[derive(Debug, Clone)]
pub enum ToNetMsg {
    /// Request termination of the network thread.
    Terminate,
    NewMsg(Message)
}

#[derive(Debug, Clone)]
pub struct Message {
    msg: String,
    sender_id: Id,
    sender_name: String,
    timestamp: SystemTime,
    is_myself: bool,
}

impl Message {
    fn new(msg: String, sender_id: Id, sender_name: String, is_myself: bool) -> Self {
        Message {
            msg: msg,
            sender_id: sender_id,
            sender_name: sender_name,
            timestamp: SystemTime::now(),
            is_myself: is_myself,
        }
    }
    pub fn get_message(&self) -> &String {
        &self.msg
    }
    pub fn get_sender_id(&self) -> Id {
        self.sender_id
    }
    pub fn get_sender_name(&self) -> &String {
        &self.sender_name
    }
    pub fn get_timestamp(&self) -> SystemTime {
        self.timestamp
    }
    pub fn is_myself(&self) -> bool {
        self.is_myself
    }
}
