
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
    empty: bool,
}

impl<'a> LookupSession<'a> {
    /// creates a session for looking up boot node addresses from a given tracker.
    pub fn new(sock: &UdpSocket, track: SocketAddr, room: Id) -> LookupSession {
        LookupSession {
            sock: sock,
            adr: track,
            id: room,
            last_lookup: 0,
            empty: false
        }
    }

}

impl<'a> Iterator for LookupSession<'a> {
    type Item = Result<SocketAddr>;

    /// returns Ok(adr) with the next address from the tracker
    /// Err(NetworkError::Timeout) if the tracker isn't responding
    /// Err(_) if a severe network error occured
    /// returns None if the last thing was an error or if there aren't any more
    /// addresses from the tracker.
    fn next(&mut self) -> Option<Self::Item> {
        if self.empty {
            return None;
        }
        let if_lookup = |r: &TrackResp| {r.is_lookup()};

        let q = TrackQuery::Lookup{id: self.id, last_lookup: self.last_lookup};
        let resp = send_with_response(self.sock, &q, self.adr, 3, Duration::from_millis(50), if_lookup);

        match resp {
            Err(e) => {
                self.empty = true;
                return Some(Err(e));
            },
            Ok(TrackResp::LookupAns{adr, lookup_id}) => {
                if let Some(a) = adr {
                    self.last_lookup = lookup_id;
                    return Some(Ok(a));
                }
                self.empty = true;
                return None;
            },
            Ok(_) => {
                unreachable!();
            }
        }
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
    let resp = send_with_response(sock, &q, tracker, 3, Duration::from_millis(50), if_update)?;

    if let TrackResp::UpdateSuccess{ttl, ..} = resp {
        return Ok(ttl);
    } else {
        unreachable!("if_update must be incorrect!!");
    }
    // Err(NetworkError::Other("update api failed for some reason (should never happen)"))
}
