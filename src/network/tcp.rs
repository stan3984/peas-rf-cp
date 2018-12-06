
use std::net::{TcpListener,SocketAddr,ToSocketAddrs,TcpStream};
use bincode::{serialize,deserialize,serialized_size};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use std::io::{self,Write};
use super::*;

/// open a TCP listener on any port as non-blocking
pub fn open_any() -> Result<TcpListener> {
    let my_adr = super::find_internet_interface()?;
    let list = TcpListener::bind(super::from_ipv4(my_adr, 0))?;
    list.set_nonblocking(true)?;
    Ok(list)
}

/// this is basically a wrapper around UdpSocket::send_to that takes something that is
/// serializable instead of a slice of bytes.
pub fn send<T>(stream: &mut TcpStream, msg: &T) -> Result<usize>
where T: Serialize
{
    let seri = serialize(msg).expect("could not serialize msg");
    Ok(stream.write(seri.as_slice())?)
}

/// tries to read ONE packet from the socket
/// doesn't set its own rules for the socket
/// returns: Ok(message) if a message was found/received
///          Err(NetworkError::NoMessage) if the message received was not what we expected
///          Err(NetworkError::Timeout) if it timed out or if socket is in nonblocking and was empty
///          Err(NetworkError::IOError(e)) if a serious error occured
pub fn recv_once<T>(stream: &mut TcpStream) -> Result<T>
where T: DeserializeOwned
{
    let mut buf = Vec::new();
    let _read = stream.read_to_end(&mut buf)
        .map_err(|e| {
            if let io::ErrorKind::WouldBlock = e.kind() {
                NetworkError::Timeout
            } else if let io::ErrorKind::TimedOut = e.kind() {
                NetworkError::Timeout
            } else {
                NetworkError::from(e)
            }
        })?;
    let de = match deserialize(&buf[..]) {
        Ok(res) => res,
        Err(_) => {
            warn!("TCP: received a message that couldn't be deserialized");
            return Err(NetworkError::NoMessage)
        },
    };

    Ok(de)
}

