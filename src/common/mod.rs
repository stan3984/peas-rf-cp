pub mod id;
pub mod logger;
pub mod timer;
use rand::RngCore;

/// get random u64 hash
pub fn get_hash() -> u64 {
    let mut rng = rand::thread_rng();
    rng.next_u64()
}
