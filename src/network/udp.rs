
use std::net::{SocketAddr,UdpSocket,ToSocketAddrs};
use std::collections::HashMap;
use serde::ser::Serialize;
use serde::de::DeserializeOwned;
use bincode::{serialize,deserialize};
use std::io;
use super::*;
use std::time::{Duration,Instant};
use std::collections::BinaryHeap;

/// thing that `send_with_responses` uses to store timeouts
/// for each connection. Sorts by the Instant in reverse order
struct ResponseTime(Instant, u32, SocketAddr);

impl std::cmp::PartialEq for ResponseTime {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ResponseTime {}

impl std::cmp::PartialOrd for ResponseTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for ResponseTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0)
    }
}

/// using sock, sends all messages from `msgs` to their destinations. `msgs` maps the destination to the message to send.
/// each destination gets `retries` amount of retries before that giving up on that destination (it will actually add it anyway if it arrives afterwards)
/// each of the destinations retries gets `timeout` amount of time before trying again
/// `pred` is run on a received message from an expected destination. The message
/// is considered trash/pretends that it never arrived if `pred` returns false.
/// The return value is Ok(hashmap) mapping destination addresses to their respective received message. An missing entry from the hashmap means that connection timed out.
pub fn send_with_responses<S, D, F>(sock: &UdpSocket, msgs: &HashMap<SocketAddr, &S>, retries: u32, timeout: Duration, pred: F) -> Result<HashMap<SocketAddr, D>>
where S: Serialize,
      D: DeserializeOwned,
      F: Fn(SocketAddr,&D) -> bool
{
    if msgs.is_empty() {
        panic!("msgs in send_with_responses is empty");
    }
    set_timeout(sock, timeout/10)?;
    let mut res = HashMap::new();
    let mut pending = BinaryHeap::new();
    for (adr, m) in msgs.iter() {
        pending.push(ResponseTime(Instant::now(), 1, *adr));
        send(&sock, m, adr)?;
    }

    'outer:
    loop {
        loop {
            if pending.is_empty() {
                break 'outer;
            }
            if res.contains_key(&pending.peek().unwrap().2) {
                pending.pop();
            } else if Instant::now().duration_since(pending.peek().unwrap().0) >= timeout {
                let mut oldest = pending.pop().unwrap();
                if oldest.1 < retries {
                    oldest.0 = Instant::now();
                    oldest.1 += 1;
                    send(&sock, &msgs.get(&oldest.2), oldest.2)?;
                    pending.push(oldest);
                }
            } else {
                break;
            }
        }
        match recv_once(sock) {
            Ok((sender, data)) => {
                if msgs.contains_key(&sender) && !res.contains_key(&sender) && pred(sender, &data) {
                    res.insert(sender, data);
                }
                debug!("pred failed");
            },
            Err(NetworkError::NoMessage) | Err(NetworkError::Timeout) => (),
            Err(ioerror) => return Err(ioerror),
        }
    }
    Ok(res)
}

/// sends `msg` to `dst` and waits for a response with type `U`.
/// only a response that fulfills `pred` is accepted
/// attempts this `retries` times before returning NetworkError::Timeout
pub fn send_with_response<T, U, F>(sock: &UdpSocket, msg: &T, dst: SocketAddr, retries: u32, total_time: Duration, pred: F) -> Result<U>
where T: Serialize,
      U: DeserializeOwned,
      F: Fn(&U) -> bool
{
    let from_same = |sender, msg: &U| sender == dst && pred(msg);
    let mut tryy = 1;
    loop {
        send(sock, msg, &dst)?;
        match recv_until_timeout(sock, total_time, from_same) {
            Ok((_, x)) => return Ok(x),
            Err(NetworkError::Timeout) => (),
            Err(ioerror) => return Err(ioerror),
        }
        if tryy >= retries {
            return Err(NetworkError::Timeout)
        }
        tryy += 1;
    }
}

/// open a socket on any port for udp
pub fn open_any() -> Result<UdpSocket> {
   let my_adr = super::find_internet_interface()?;
   Ok(UdpSocket::bind(super::from_ipv4(my_adr, 0))?)
}

/// this is basically a wrapper around UdpSocket::send_to that takes something that is
/// serializable instead of a slice of bytes.
pub fn send<T, A>(sock: &UdpSocket, msg: &T, to: A) -> Result<usize>
where T: Serialize,
      A: ToSocketAddrs
{
    Ok(sock.send_to(serialize(msg).expect("could not serialize msg").as_slice(), to)?)
}

/// tries to read ONE packet from the socket
/// doesn't set its own rules for the socket
/// returns: Ok((sender, message)) if a message was found/received
///          Err(NetworkError::NoMessage) if the message received was not what we expected
///          Err(NetworkError::Timeout) if it timed out or if socket is in nonblocking and was empty
///          Err(NetworkError::IOError(e)) if a serious error occured
pub fn recv_once<T>(sock: &UdpSocket) -> Result<(SocketAddr, T)>
where T: DeserializeOwned
{
    let mut buf = [0; 512];
    let (read, sender) = sock.recv_from(&mut buf)
        .map_err(|e| {
            if let io::ErrorKind::WouldBlock = e.kind() {
                NetworkError::Timeout
            } else if let io::ErrorKind::TimedOut = e.kind() {
                NetworkError::Timeout
            } else {
                NetworkError::from(e)
            }
        })?;
    if read >= buf.len() {
        warn!("received a message that was too big {} == {}", read, buf.len());
        return Err(NetworkError::NoMessage);
    }

    let de = match deserialize(&buf[..read]) {
        Ok(res) => res,
        Err(_) => {
            warn!("received a message that couldn't be deserialized");
            return Err(NetworkError::NoMessage)
        },
    };

    Ok((sender, de))
}

/// runs `recv_once` until it returns something successful
/// sets sock to blocking
pub fn recv_until_msg<T>(sock: &UdpSocket) -> Result<(SocketAddr, T)>
where T: DeserializeOwned
{
    set_blocking(sock)?;
    loop {
        match recv_once(sock) {
            Ok(x) => return Ok(x),
            Err(NetworkError::NoMessage) | Err(NetworkError::Timeout) => (),
            ioerror => return ioerror,
        }
    }
}

/// runs `recv_once` over and over up to `timeout` seconds.
/// changes settings on sock
/// returns NetworkError::Timeout if `timeout` ran out (not exact!)
/// only returns an Ok if `pred` returns true on the received message
pub fn recv_until_timeout<T, F>(sock: &UdpSocket, timeout: Duration, pred: F) -> Result<(SocketAddr, T)>
where T: DeserializeOwned,
      F: Fn(SocketAddr,&T) -> bool
{
    set_timeout(sock, timeout/10)?;
    let start = Instant::now();
    loop {
        match recv_once(sock) {
            Ok((sender, data)) => {
                if pred(sender,&data) {
                    return Ok((sender, data))
                }
                debug!("pred failed");
            },
            Err(NetworkError::NoMessage) | Err(NetworkError::Timeout) => (),
            ioerror => return ioerror,
        }
        if Instant::now().duration_since(start) >= timeout {
            return Err(NetworkError::Timeout);
        }
    }
}

pub fn set_nonblocking(sock: &UdpSocket) -> io::Result<()> {
    sock.set_nonblocking(true)
}

/// set the specified timeout on the socket
pub fn set_timeout(sock: &UdpSocket, dur: Duration) -> io::Result<()> {
    sock.set_nonblocking(false)?;
    sock.set_read_timeout(Some(dur))?;
    Ok(())
}

/// set socket to blocking
pub fn set_blocking(sock: &UdpSocket) -> io::Result<()> {
    sock.set_nonblocking(false)?;
    sock.set_read_timeout(None)?;
    Ok(())
}
