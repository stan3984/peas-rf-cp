
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use super::*;
use common::id::Id;
use network::udp::*;
use network::Result;

pub fn lookup(sock: &UdpSocket, tracker: SocketAddr, id: Id, last_lookup: u32) -> Result<Option<(SocketAddr, u32)>> {
    let if_lookup = |r: &TrackResp| {r.is_lookup()};
    let q = TrackQuery::Lookup{id: id, last_lookup: last_lookup};
    let resp = send_with_response(sock, &q, tracker, 3, Duration::from_millis(500), if_lookup)?;

    if let TrackResp::LookupAns{adr: recv_adr, lookup_id: recv_id} = resp {
        if let Some(adr) = recv_adr {
            return Ok(Some((adr, recv_id)));
        }
    } else {
        // TODO: log, shouldn't happen
    }
    Ok(None)
}
