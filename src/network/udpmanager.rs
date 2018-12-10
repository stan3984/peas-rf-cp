
use std::net::{SocketAddr,UdpSocket};
use std::sync::mpsc::{Sender,Receiver,channel,TryRecvError};
use common::timer::Timer;
use std::collections::HashMap;
use std::thread;
use network::udp;
use super::*;
use std::time::Duration;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use bincode::{deserialize, serialize};
use rand::RngCore;
use std::slice::Iter;
use common::get_hash;

const TICKET_TTL: Duration = Duration::from_millis(50);
const SLEEP_TIME: Duration = Duration::from_millis(20);
const RETRIES: u32 = 3;

/// manager that can handle multiple active sessions over
/// one UDP socket. This starts in a new thread.
/// A "session" is a one packet request to another node
/// and a one packet response.
/// Things that can respond to requests are called Services
/// and active sessions are called Tickets
pub struct Manager {
    to_man: Sender<Request>,
    sock: UdpSocket,
}

/// instructions that can be sent to a Manager
enum Request {
    /// request to activate a new session
    Send(Ticket),
    Service(Service),
    Terminate,
}

/// results from a `Ticket` sent from the manager thread.
struct TicketResponse {
    payload: Option<Vec<u8>>,
    source: SocketAddr,
}

/// a request to a service
struct ServiceResponse {
    payload: Vec<u8>,
    source: SocketAddr,
    id: u64,
}

/// holds the necessary info for an active session
struct Ticket {
    id: u64,
    timer: Timer,
    retries: u32,
    requester: Sender<TicketResponse>,
    payload: Vec<u8>,
    dest: SocketAddr,
    service: u32,
}

/// holds the necessary info for a service
struct Service {
    service: u32,
    pipe: Sender<ServiceResponse>,
}

/// the entry point for interacting with a session.
pub struct SendHandle<T> {
    /// channel where finished tickets are sent
    rec: Receiver<TicketResponse>,
    /// how many responses we are waiting for
    count: u32,
    /// all destinations
    all: Vec<SocketAddr>,
    /// desinations mapped to their responses
    responses: HashMap<SocketAddr, T>,
}

/// the entry point for interacting with a local service
pub struct ServiceHandle {
    rec: Receiver<ServiceResponse>,
    sock: UdpSocket,
}

#[derive(Serialize, Deserialize)]
/// the actual struct that is sent between nodes
struct Msg {
    service: u32,
    id: u64,
    payload: Vec<u8>,
}

impl Manager {
    /// starts a new manager on `sock`
    pub fn start(sock: UdpSocket) -> Self {
        let (tx, rx) = channel();
        let sock_clone = sock.try_clone().unwrap();
        thread::spawn(move || {
            manager_main(rx, sock_clone);
        });
        Manager{to_man: tx, sock: sock}
    }
    pub fn terminate(self) {
        info!("Udp Manager is terminating as per request...");
        self.to_man.send(Request::Terminate).unwrap();
    }
    /// takes a manager and creates a new service with it
    pub fn register_service(&self, service: u32) -> ServiceHandle {
        let (tx, rx) = channel();
        let servh = ServiceHandle{rec: rx, sock: self.sock.try_clone().unwrap()};
        let ser = Service{service: service, pipe: tx};
        self.to_man.send(Request::Service(ser)).unwrap();
        servh
    }
}

/// takes a service and receives a request from it.
/// returning the message, source and session id
pub fn service_get<T>(servh: &ServiceHandle) -> Option<(T, SocketAddr, u64)>
where T: DeserializeOwned
{
    servh
        .rec
        .try_recv()
        .map_err(|e| if e == TryRecvError::Disconnected {
            panic!("udpmanager disconnected")
        })
        .ok()
        .and_then(|sr| deserialize(&sr.payload)
                  .map_err(|_| warn!("service got a message that couldn't be deserialized"))
                  .ok()
                  .map(|de| (de, sr.source, sr.id)))
}

/// respond to a request to a service
pub fn service_respond<T>(servh: &ServiceHandle, resp: &T, id: u64, to: SocketAddr) -> Result<()>
where T: Serialize
{
    let resp_serialized = serialize(resp).expect("could not serialize resp");
    let to_send = Msg{
        service: 0,
        id: id,
        payload: resp_serialized,
    };
    udp::send(&servh.sock, &to_send, to)?;
    Ok(())
}

/// initiates a new session. Sending `msg` to all `dests` to a service `service`
pub fn send<T,U>(man: &Manager, msg: &T, dests: Vec<SocketAddr>, service: u32) -> SendHandle<U>
where U: DeserializeOwned,
      T: Serialize
{
    assert!(!dests.is_empty(), "dests is empty");
    let (tx, rx) = channel();
    let seri = serialize(msg).expect("couldn't serialize");
    let id = get_hash();

    for d in dests.iter() {
        let t = Ticket{
            id: id,
            timer: Timer::new_expired(),
            retries: RETRIES,
            requester: tx.clone(),
            payload: seri.clone(),
            dest: *d,
            service: service,
        };
        man.to_man.send(Request::Send(t)).unwrap();
    }
    SendHandle{
        rec: rx,
        count: dests.len() as u32,
        all: dests,
        responses: HashMap::new()
    }
}

impl<T> SendHandle<T>
where T: DeserializeOwned
{
    fn process_response(&mut self, tickresp: TicketResponse) {
        let TicketResponse{payload, source} = tickresp;
        self.count -= 1;
        if payload.is_some() {
            let des = deserialize(&payload.unwrap()[..]);
            if des.is_err() {
                error!("SendHandle got a message of the wrong kind or checksums aren't used");
            } else {
                self.responses.insert(source, des.unwrap());
            }
        }
    }
    /// blocks until this session has finished
    pub fn update_wait(&mut self) {
        loop {
            if self.count == 0 {
                break;
            }
            match self.rec.recv() {
                Ok(tr) => self.process_response(tr),
                Err(_) => {
                    if self.count != 0 {
                        error!("some tickets died early?");
                    }
                    break;
                }
            }
        }
    }
    /// makes the handle read results from the manager and updates itself
    pub fn update(&mut self) {
        loop {
            if self.count == 0 {
                break;
            }
            match self.rec.try_recv() {
                Ok(tr) => self.process_response(tr),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.count != 0 {
                        error!("some tickets died early?");
                    }
                    break;
                }
            }
        }
    }
    /// is this session done?
    pub fn is_done(&self) -> bool {
        self.count == 0
    }
    /// gets an Iter over all destination addresses
    pub fn iter(&self) -> Iter<SocketAddr> {
        assert!(self.is_done());
        self.all.iter()
    }
    /// returns true if `a` didn't respond or responded with something that couldn't be deserialized to T
    pub fn is_dead(&self, a: &SocketAddr) -> bool {
        assert!(self.is_done());
        !self.responses.contains_key(a)
    }
    /// extracts the answer from `a`
    pub fn get_answer(&mut self, a: &SocketAddr) -> T {
        assert!(self.is_done());
        self.responses.remove(a).expect("tried to get an answer of something that is dead")
    }
    pub fn borrow_answer(&self, a: &SocketAddr) -> &T {
        assert!(self.is_done());
        self.responses.get(a).expect("tried to borrow an answer of something that is dead")
    }
    /// convenience function to extract a single answer from this handle
    /// A return value of None means the same things as `is_dead` == true
    pub fn get_single_answer(mut self) -> Option<T> {
        assert!(self.all.len() == 1, "this SendHandle had more than one destination");
        self.responses.remove(&self.all[0])
    }
}

/// the main function of the manager thread
fn manager_main(recv: Receiver<Request>, sock: UdpSocket) {
    let mut services = Vec::new();
    let mut tickets = Vec::new();
    udp::set_nonblocking(&sock).unwrap();

    loop {
        // read new stuff for the manager
        match recv.try_recv() {
            Ok(Request::Send(tick)) => {
                tickets.push(tick);
            }
            Ok(Request::Service(ser)) => {
                services.push(ser);
            }
            Ok(Request::Terminate) => {
                break;
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                error!("master died before us");
                break;
            }
        }

        // receive new things
        // TODO: timeout on this?
        loop {
            let (sender, msg): (_, Msg) = match udp::recv_once(&sock) {
                Ok(x) => x,
                Err(NetworkError::NoMessage) => continue,
                Err(NetworkError::Timeout) => break,
                Err(ioerror) => panic!(ioerror),
            };

            // was sent to a service
            if msg.service != 0 {
                for s in services.iter() {
                    if msg.service == s.service {
                        s.pipe.send(ServiceResponse{
                            payload: msg.payload,
                            source: sender,
                            id: msg.id
                        }).unwrap();
                        break;
                    }
                }
            } else { // was a response to a ticket
                for i in (0..tickets.len()).rev() {
                    if tickets[i].id == msg.id && sender == tickets[i].dest {
                        tickets[i].requester.send(TicketResponse{
                            payload: Some(msg.payload),
                            source: sender
                        }).unwrap();
                        tickets.remove(i);
                        break;
                    }
                }
            }
        }

        // see if a ticket needs to be sent again
        for i in (0..tickets.len()).rev() {
            if tickets[i].timer.expired(1.0) {
                if tickets[i].retries == 0 {
                    debug!("a ticket expired");
                    tickets[i].requester.send(TicketResponse{
                        payload: None,
                        source: tickets[i].dest
                    }).unwrap();
                    tickets.remove(i);
                } else {
                    if tickets[i].timer.get_timeout() == Duration::from_millis(0) {
                        debug!("sending a ticket");
                    } else {
                        debug!("resending a ticket");
                    }
                    tickets[i].retries -= 1;
                    tickets[i].timer.reset_with(TICKET_TTL);
                    send_msg(&sock, tickets[i].id, tickets[i].service, &tickets[i].payload, tickets[i].dest);
                }
            }
        }

        thread::sleep(SLEEP_TIME);
    }
    info!("Udp Manager terminated");
}

fn send_msg(sock: &UdpSocket, id: u64, service: u32, payload: &Vec<u8>, dest: SocketAddr) {
    udp::send(sock, &Msg{id: id, service: service, payload: payload.clone()}, dest).unwrap();
}

