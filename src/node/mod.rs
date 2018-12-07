pub mod nethandle;
mod ktable;
mod netthread;
mod kademlia;
mod cache;

use std::net::SocketAddr;
use common::id::Id;
use std::time::SystemTime;
use network::tcp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpBroadcast {
    hash: u64,
    payload: TcpPayload,
}

impl TcpBroadcast {
    pub fn new(pay: TcpPayload) -> Self {
        TcpBroadcast{hash: tcp::get_hash(), payload: pay}
    }
    pub fn from_message(msg: Message) -> Self {
        TcpBroadcast::new(TcpPayload::Msg(msg))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TcpPayload {
    IsAlive(Id),
    Msg(Message),
}

#[derive(Debug, Clone)]
pub enum FromNetMsg {
    Error(Option<String>),
    NewMsg(Message),
}

impl FromNetMsg {
    pub fn from_message(msg: Message) -> Self {
        FromNetMsg::NewMsg(msg)
    }
}

#[derive(Debug, Clone)]
pub enum ToNetMsg {
    /// Request termination of the network thread.
    Terminate,
    NewMsg(Message)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
