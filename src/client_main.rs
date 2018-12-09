extern crate bincode;

extern crate clap;
use clap::{App, Arg, ArgMatches};

extern crate log;
use log::LevelFilter;

extern crate peas_rf_cp;
use peas_rf_cp::common::id::Id;
use peas_rf_cp::common::logger;
use peas_rf_cp::node::nethandle::NetHandle;

use std::fs::File;
use std::io::{self, Read, Write};
use std::mem;
use std::net::{ToSocketAddrs,SocketAddr};

const ARG_USERNAME: &str = "username";
const ARG_LOG_LEVEL: &str = "log-level";
const ARG_LOG_STDERR: &str = "log-stderr";
const ARG_NEW_ROOM: &str = "new-room";
const ARG_JOIN_ROOM: &str = "join-room";
const ARG_TRACKER: &str = "tracker";

fn main() {
    let app = create_app();
    let matches = app.get_matches();

    setup_logging(&matches);

    if matches.is_present(ARG_NEW_ROOM) {
        match create_room(&matches) {
            Ok(_) => {},
            Err(x) => log::error!("Failed to create room ({})", x),
        }
    } else if matches.is_present(ARG_JOIN_ROOM) {
        match parse_room(&matches) {
            Ok(room_id) => {
                run(matches.value_of(ARG_USERNAME).unwrap().to_string(),
                    room_id,
                    matches.value_of(ARG_TRACKER).unwrap().to_string());
            },
            Err(x) => log::error!("Failed to parse room ({})", x),
        }
    }

    log::info!("Shutting down");
}

fn run(username: String, room_id: Id, tracker: String) {
    let neth = NetHandle::new(true,
                              Id::from_u64(0),
                              username,
                              room_id,
                              tracker.to_socket_addrs().unwrap().collect());
    let mut asdasd = 1;
    loop {
        // neth.send_message(asdasd.to_string()).unwrap();
        asdasd += 1;
        loop {
            match neth.read() {
                Ok(Some(_)) => (),
                Ok(None) => break,
                Err(_) => panic!(),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5000));
    }
}

fn setup_logging<'a>(matches: &ArgMatches<'a>) {
    let level = match matches.value_of(ARG_LOG_LEVEL) {
        Some("all") => LevelFilter::max(),
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("info") => LevelFilter::Info,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        None => LevelFilter::Off,
        Some(_) => unreachable!(),
    };

    let to_file = !matches.is_present(ARG_LOG_STDERR);
    logger::initialize_logger(level, to_file);
}

fn create_room<'a>(matches: &ArgMatches<'a>) -> io::Result<()> {
    let new_room = matches.value_of(ARG_NEW_ROOM);
    assert!(new_room.is_some());

    let room_name = new_room.unwrap();
    assert!(room_name.len() > 0);

    let room_id = Id::new_random();
    let file_name = format!("{}.peas-room", room_name);

    let mut file = File::create(&file_name)?;
    file.write(&bincode::serialize(&room_id).unwrap()[..])?;

    log::debug!("Created room file `{}`", file_name);

    Ok(())
}

fn parse_room<'a>(matches: &ArgMatches<'a>) -> io::Result<Id> {
    let join_room = matches.value_of(ARG_JOIN_ROOM);
    assert!(join_room.is_some());

    let room_file = join_room.unwrap();

    let mut file = File::open(room_file)?;

    let id = {
        // note: this code can be simplified but is kept this way in
        // case we want to write/read more data than just the id

        let mut buffer = vec![0; mem::size_of::<Id>()];

        let room_id = {
            file.read(&mut buffer)?;
            bincode::deserialize::<Id>(&buffer).unwrap()
        };

        log::debug!("Parsed room with id `{}`", room_id);
        room_id
    };

    Ok(id)
}

fn create_app<'a, 'b>() -> App<'a, 'b> {
    let a = App::new("peas-rf-cp")
        .version("0.0.0-alpha")
        .arg(
            Arg::with_name(ARG_USERNAME)
                .long("username")
                .short("u")
                .help("Username to display")
                .conflicts_with_all(&["new-room"])
                .takes_value(true)
                .requires_all(&[ARG_JOIN_ROOM, ARG_TRACKER]),
        ).arg(
            Arg::with_name(ARG_LOG_LEVEL)
                .long("log")
                .help("Logging level")
                .takes_value(true)
                .possible_values(&["all", "trace", "debug", "info", "warn", "error"]),
        ).arg(
            Arg::with_name(ARG_LOG_STDERR)
                .long("log-stderr")
                .help("Directs logging output to standard error (default is logging to file)")
                .requires_all(&[ARG_LOG_LEVEL]),
        ).arg(
            Arg::with_name(ARG_NEW_ROOM)
                .long("new-room")
                .help("Creates a new room file and exits")
                .takes_value(true)
                .conflicts_with_all(&[ARG_USERNAME, ARG_JOIN_ROOM]),
        ).arg(
            Arg::with_name(ARG_JOIN_ROOM)
                .long("join")
                .short("j")
                .help("Specifies the room to join")
                .takes_value(true)
                .conflicts_with_all(&[ARG_NEW_ROOM])
                .requires_all(&[ARG_USERNAME, ARG_TRACKER]),
        ).arg(
            Arg::with_name(ARG_TRACKER)
                .long("tracker")
                .short("t")
                .help("Specifies the tracker to connect to")
                .takes_value(true)
                .requires_all(&[ARG_JOIN_ROOM, ARG_USERNAME]),
        );

    return a;
}
