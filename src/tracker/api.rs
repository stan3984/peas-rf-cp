
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use super::*;
use common::id::Id;
use network::udp::*;
use network::Result;
use network::NetworkError;

pub struct LookupSession<'a> {
    sock: &'a UdpSocket,
    adr: SocketAddr,
    id: Id,
    last_lookup: u32,
}

impl<'a> LookupSession<'a> {
    /// creates a session for looking up boot node addresses from a given tracker.
    pub fn new(sock: &UdpSocket, track: SocketAddr, room: Id) -> LookupSession {
        LookupSession {
            sock: sock,
            adr: track,
            id: room,
            last_lookup: 0,
        }
    }

    /// behaves like an iterator, returns Ok(None) if the tracker doesn't have any more entries
    /// Err(NetworkError::Timeout) if the tracker isn't responding
    /// Err(_) if a severe network error occured
    pub fn next(&mut self) -> Result<Option<SocketAddr>> {
        let if_lookup = |r: &TrackResp| {r.is_lookup()};

        let q = TrackQuery::Lookup{id: self.id, last_lookup: self.last_lookup};
        let resp = send_with_response(self.sock, &q, self.adr, 3, Duration::from_millis(500), if_lookup)?;

        if let TrackResp::LookupAns{adr: recv_adr, lookup_id: recv_id} = resp {
            if let Some(adr) = recv_adr {
                self.last_lookup = recv_id;
                return Ok(Some(adr));
            }
        } else {
            error!("if_lookup must be incorrect!!");
        }
        Ok(None)
    }
}

/// updates the room `room` at tracker `tracker` using `sock`. `my_adr` is the address
/// the tracker should add to its database
/// returns Ok(ttl) which is the amount of time the entry will stay in the tracker
/// Err(NetworkError::Timeout) if the tracker isn't responding
/// Err(_) for something else
pub fn update(sock: &UdpSocket, room: Id, my_adr: SocketAddr, tracker: SocketAddr) -> Result<Duration> {
    let if_update = |r: &TrackResp| {r.is_update()};

    let q = TrackQuery::Update{id: room, adr: my_adr};
    let resp = send_with_response(sock, &q, tracker, 3, Duration::from_millis(500), if_update)?;

    if let TrackResp::UpdateSuccess{ttl, ..} = resp {
        return Ok(ttl);
    } else {
        error!("if_update must be incorrect!!");
    }
    Err(NetworkError::Other("update api failed for some reason (should never happen)"))
}
