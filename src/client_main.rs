extern crate bincode;

extern crate clap;
use clap::{App, Arg, ArgMatches};

extern crate log;
use log::LevelFilter;

extern crate peas_rf_cp;
use peas_rf_cp::common::id::Id;
use peas_rf_cp::common::logger;

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::mem;

const ARG_USERNAME: &'static str = "username";
const ARG_LOG_LEVEL: &'static str = "log-level";
const ARG_LOG_STDERR: &'static str = "log-stderr";
const ARG_NEW_ROOM: &'static str = "new-room";
const ARG_JOIN_ROOM: &'static str = "join-room";
const ARG_TRACKER: &'static str = "tracker";

fn main() {
    let matches = get_arguments();

    setup_logging(&matches);

    if matches.is_present(ARG_NEW_ROOM) {
        match create_room(&matches) {
            Ok(_) => {}
            Err(x) => log::error!("Failed to create room ({})", x),
        }
    } else if matches.is_present(ARG_JOIN_ROOM) {
        parse_room(&matches);
    }

    log::info!("Shutting down");
}

fn setup_logging<'a>(matches: &ArgMatches<'a>) {
    let level = match matches.value_of(ARG_LOG_LEVEL) {
        Some("all") => LevelFilter::max(),
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("info") => LevelFilter::Info,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        Some("off") => LevelFilter::Off,
        _ => unreachable!(),
    };

    let to_file = !matches.is_present(ARG_LOG_STDERR);
    logger::initialize_logger(level, to_file);
}

fn create_room<'a>(matches: &ArgMatches<'a>) -> io::Result<()> {
    let new_room = matches.value_of(ARG_NEW_ROOM);
    assert!(new_room.is_some());

    let room_name = new_room.unwrap();
    assert!(room_name.len() > 0);

    log::trace!(
        "Attempting to create room file for new room `{}`",
        room_name
    );

    let room_id = Id::new_random();

    let file_name = format!("{}.peas-room", room_name);

    let mut file = File::create(&file_name)?;
    file.write(&bincode::serialize(&room_id).unwrap()[..]);
    // file.write(&bincode::serialize(&room_name).unwrap()[..]);

    log::trace!("Successfully created room file `{}`", file_name);

    Ok(())
}

fn parse_room<'a>(matches: &ArgMatches<'a>) -> io::Result<Id> {
    let join_room = matches.value_of(ARG_JOIN_ROOM);
    assert!(join_room.is_some());

    let room_file = join_room.unwrap();

    log::debug!("Reading from file `{}`", room_file);

    let mut file = File::open(room_file)?;
    file.seek(SeekFrom::Start(0));

    let id = {
        // note: this code can be simplified but is kept this way in
        // case we want to write/read more data than just the id

        let mut buffer = vec![0; mem::size_of::<Id>()];

        let room_id = {
            file.read(&mut buffer)?;
            bincode::deserialize::<Id>(&buffer).unwrap()
        };

        // let room_name = {
        //     buffer.clear();
        //     file.read_to_end(&mut buffer);
        //     bincode::deserialize::<String>(&buffer).unwrap()
        // };

        log::debug!("Parsed room with id `{}`", room_id);
        room_id
    };

    Ok(id)
}

fn get_arguments<'a>() -> ArgMatches<'a> {
    let a = App::new("peas-rf-cp")
        .version("0.0.0-alpha")
        .arg(
            Arg::with_name(ARG_USERNAME)
                .long("username")
                .short("u")
                .help("Username to display")
                .conflicts_with_all(&["new-room"])
                .takes_value(true)
                .requires_all(&[ARG_JOIN_ROOM]),
        ).arg(
            Arg::with_name(ARG_LOG_LEVEL)
                .long("log")
                .help("Logging level")
                .possible_values(&["all", "trace", "debug", "info", "warn", "error", "off"])
                .default_value("off"),
        ).arg(
            Arg::with_name(ARG_LOG_STDERR)
                .long("log-stderr")
                .help("Directs logging output to standard error (default is logging to file)"),
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
                .requires_all(&[ARG_USERNAME]),
        ).arg(
            Arg::with_name(ARG_TRACKER)
                .long("tracker")
                .short("t")
                .help("Specifies the tracker to connect to")
                .takes_value(true)
                .requires_all(&[ARG_JOIN_ROOM]),
        );

    a.get_matches()
}
