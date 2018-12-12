use std::ops::Add;
use std::thread;
use std::time::{Duration, Instant};

use log;
use rand::{thread_rng, Rng};

use node::nethandle::NetHandle;

pub fn bot_main(neth: NetHandle) {
    const MIN_WAIT_MS: u64 = 1_000;
    const MAX_WAIT_MS: u64 = 10_000;
    const SLEEP_MS: u64 = 500;

    let mut cnt = 0;
    let mut rng = thread_rng();

    let mut next_action = Instant::now();

    loop {
        loop {
            match neth.read() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(e) => {
                    log::error!("{:?}", e);
                    std::process::exit(1);
                }
            }
        }

        let now = Instant::now();
        if now >= next_action {
            cnt += 1;
            let message = cnt.to_string();
            log::debug!("BOT: sending message `{}`", message);

            match neth.send_message(message) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{:?}", e);
                    std::process::exit(1);
                }
            }

            let delay = Duration::from_millis(rng.gen_range(MIN_WAIT_MS, MAX_WAIT_MS));
            log::debug!("BOT: next action in {:?}", delay);
            next_action = now.add(delay);
        }

        thread::sleep(Duration::from_millis(SLEEP_MS));
    }
}
