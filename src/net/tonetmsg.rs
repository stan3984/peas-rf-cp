use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub enum ToNetMsg {
    /// Request termination of the network thread.
    Terminate,
    /// Update the username for this client.
    SetUsername(String),
    /// Add a new tracker to our list of known trackers.
    RegisterTracker(SocketAddr),
    /// Removes a tracker from our list of known trackers.
    UnregisterTracker(SocketAddr),
}
