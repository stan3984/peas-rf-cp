extern crate clap;
extern crate log;

extern crate peas_rf_cp;

use log::LevelFilter;

use clap::{App, Arg, ArgMatches};

use peas_rf_cp::common::logger;
use peas_rf_cp::common::id::Id;

const ARG_USERNAME: &'static str = "username";
const ARG_LOG_LEVEL: &'static str = "log-level";
const ARG_LOG_STDERR: &'static str = "log-stderr";
const ARG_NEW_ROOM: &'static str = "new-room";
const ARG_JOIN_ROOM: &'static str = "join-room";
const ARG_TRACKER: &'static str = "tracker";

fn main() {
    let matches = get_arguments();

    setup_logging(&matches);

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
