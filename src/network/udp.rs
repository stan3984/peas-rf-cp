
use std::net::{SocketAddr,UdpSocket,ToSocketAddrs};
use std::collections::HashMap;
use serde::ser::Serialize;
use serde::de::DeserializeOwned;
use bincode::{serialize,deserialize};
use std::io;
use super::NetworkError;

pub fn send_with_responses(sock: &UdpSocket, msgs: &HashMap<SocketAddr, &[u8]>, retries: i32) -> HashMap<SocketAddr, Vec<u8>> {
   HashMap::new()
}

/// open a socket on any port for udp
pub fn open_any() -> Result<UdpSocket, NetworkError> {
   let my_adr = super::find_internet_interface()?;
   let cons = super::get_connection_candidates(my_adr, 5);
   Ok(UdpSocket::bind(&cons[..])?)
}

/// this is basically a wrapper around UdpSocket::send_to that takes something that is
/// serializable instead of a slice of bytes.
pub fn send<T, A>(sock: &UdpSocket, msg: &T, to: A) -> Result<usize, NetworkError>
where T: Serialize,
      A: ToSocketAddrs
{
    Ok(sock.send_to(serialize(msg).expect("could not serialize msg").as_slice(), to)?)
}

/// TRIES to read ONE packet from the socket
/// abides to the same rules as UdpSocket::recv_from()
pub fn recv_once<T>(sock: &UdpSocket) -> Result<(SocketAddr, T), NetworkError>
where T: DeserializeOwned
{
    let mut buf = Vec::with_capacity(512);
    let (read, sender) = sock.recv_from(&mut buf)?;
    if read >= buf.len() {
        return Err(NetworkError);
    }

    let de = match deserialize(&buf) {
        Ok(res) => res,
        Err(_) => return Err(NetworkError),
    };

    Ok((sender, de))
}

/// runs `recv_once` until it returns something successful
pub fn recv_until<T>(sock: &UdpSocket) -> Result<(SocketAddr, T), NetworkError>
where T: DeserializeOwned
{
    loop {
        match recv_once(sock) {
            Ok(x) => return Ok(x),
            // TODO: very bad, check against more specific errors
            Err(_) => (),
        }
    }
}
