
use std::net::{UdpSocket, SocketAddr};
use ::network::{Result,NetworkError};
use ::network::udp;
use tracker::api;
use ::common::id::Id;
use std::time::Duration;
use ::node::ktable::Ktable;

#[derive(Serialize, Deserialize, Debug)]
enum KadMsg {
    /// checks if another host is alive
    Ping,
    /// answer to `Ping`
    Pong(Id),
}

impl KadMsg {
    pub fn is_pong(&self) -> bool {
        if let KadMsg::Pong(_) = self {
            return true;
        }
        return false;
    }
}

/// queries all trackers and returns the first bootstrap node that is alive
pub fn find_bootstrapper(sock: &UdpSocket, room_id: Id, trackers: &Vec<SocketAddr>) -> Result<Option<(SocketAddr, Id)>> {
    'outer:
    for track in trackers.iter() {
        let sess = api::LookupSession::new(sock, *track, room_id);
        for b in sess {
            match b {
                Ok(adr) => {
                    if let Some(id) = is_alive(sock, adr)? {
                        return Ok(Some((adr, id)));
                    }
                },
                Err(NetworkError::Timeout) => continue 'outer,
                Err(e) => return Err(e),
            }
        }
    }
    Ok(None)
}

/// simply checks whether `adr` is an alive kademlia node and returns its id
pub fn is_alive(sock: &UdpSocket, adr: SocketAddr) -> Result<Option<Id>> {
    match udp::send_with_response(sock, &KadMsg::Ping, adr, 3, Duration::from_millis(500), |msg: &KadMsg| msg.is_pong()) {
        Ok(KadMsg::Pong(id)) => return Ok(Some(id)),
        Err(NetworkError::Timeout) => return Ok(None),
        Err(e) => return Err(e),
        Ok(_) => unreachable!(),
    }

}

/// handles one kademlia message
/// times out after `timeout`
pub fn handle_msg(sock: &UdpSocket, my_id: Id, timeout: Duration) -> Result<()> {
    match udp::recv_until_timeout(sock, timeout, |_,_| true) {
        Ok((sender, KadMsg::Ping)) => {
            debug!("{} pinged me!", sender);
            udp::send(sock, &KadMsg::Pong(my_id), sender)?;
            Ok(())
        },
        Err(NetworkError::Timeout) => return Ok(()),
        Err(e) => return Err(e),
        _ => Ok(()),
    }
}
