
extern crate peas_rf_cp;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;

use std::net::{UdpSocket,SocketAddr};
use std::env;
use bincode::{serialize, deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Wow {
    name: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        eprintln!("Usage: tracker (--receiver | --sender)");
        std::process::exit(1);
    }

    let sen = SocketAddr::from(([127,0,0,1], 8081));
    let rec = SocketAddr::from(([127,0,0,1], 8080));
    if args[1] == "--receiver" {
        receiver(rec);
    } else if args[1] == "--sender" {
        sender(sen, rec);
    }
}

fn sender(myself: SocketAddr, target: SocketAddr) {
    println!("sender started! I am {}", myself);

    let socket = UdpSocket::bind(SocketAddr::from(myself)).unwrap();

    let wow = Wow{ name: "omg".to_string() };

    let seri = serialize(&wow).unwrap();

    println!("I am sending {:?}", wow);

    socket.send_to(&seri, target).unwrap();
    println!("i sent some stuff to {}! (it hopefully arrived)", target);

}

fn receiver(myself: SocketAddr) {
    println!("tracker started! I am {}", myself);

    let socket = UdpSocket::bind(SocketAddr::from(myself)).unwrap();

    let mut buffer = [0; 1024];

    println!("sleeping...");
    std::thread::sleep(std::time::Duration::from_millis(10000));

    println!("receiving...");
    let (_amount, src) = socket.recv_from(&mut buffer).unwrap();

    println!("received from {}", src);

    let recv: Wow;
    match deserialize(&buffer) {
        Ok(w) => recv = w,
        Err(_) => {println!("received garbage :("); return},
    }

    println!("{:?}", recv);
}

