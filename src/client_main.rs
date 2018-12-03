extern crate clap;
extern crate log;

extern crate peas_rf_cp;

use log::LevelFilter;

use clap::{App, Arg, ArgMatches};

use peas_rf_cp::common::logger;

const ARG_USERNAME: &'static str = "username";
const ARG_LOG_LEVEL: &'static str = "log-level";
const ARG_LOG_STDERR: &'static str = "log-stderr";
const ARG_NEW_ROOM: &'static str = "new-room";
const ARG_JOIN_ROOM: &'static str = "join-room";
const ARG_TRACKER: &'static str = "tracker";

fn main() {
    let matches = get_arguments();

    setup_logging(&matches);
}

fn setup_logging<'a>(matches: &ArgMatches<'a>) {
    let level = match matches.value_of(ARG_LOG_LEVEL) {
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("info") => LevelFilter::Info,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        _ => unreachable!(),
    };

    logger::initialize_logger(level, true);
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
                .possible_values(&["trace", "debug", "info", "warn", "error"])
                .default_value("info"),
        ).arg(
            Arg::with_name(ARG_LOG_STDERR)
                .long("log-stderr")
                .help("Set logging output to standard error (default is logging to file)"),
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
