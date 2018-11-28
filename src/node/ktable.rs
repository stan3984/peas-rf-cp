
use std::net::SocketAddr;
use common::id::Id;

// varför clone?
#[derive(Copy, Clone, PartialEq, Eq)]
// TODO: implement getters
// TODO: implement new and stuff
pub struct Entry {
    sock: SocketAddr,
    // TODO: id har coola funktioner
    id: Id,
}

pub struct Ktable {}

impl Ktable {
    pub fn new(k: u32) -> Self {
        // use Id length ??
        Ktable {}
    }
    pub fn offer(&mut self, offer: Entry) {
        
    }
    pub fn delete(&mut self, entry: Entry) {
        
    }
    pub fn clear(&mut self) {
        
    }
    pub fn get(&self, n: u32) -> Vec<Entry> {
        vec![]
    }
}
