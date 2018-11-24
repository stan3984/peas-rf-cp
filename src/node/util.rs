use std::sync::mpsc::{self, Receiver, Sender};

#[inline]
pub fn new_channel_in_option<T>() -> (Option<Sender<T>>, Option<Receiver<T>>) {
    let (a, b) = mpsc::channel();
    (Some(a), Some(b))
}
