pub mod server;
pub mod api;

use common::id::Id;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
/// things that can be requested of the tracker
enum TrackQuery {
    /// update info about a room
    Update {
        /// room id to refresh
        id: Id,
        /// address where the room lives
        adr: SocketAddr,
    },
    /// check where a room exists
    Lookup {
        /// id of room to find a room for
        id: Id,
        /// lookup reference, used to make multiple requests work, start as 0
        last_lookup: u32,
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// things the tracker can respond with
enum TrackResp {
    /// TrackQuery::Update was successful
    UpdateSuccess {
        /// this room was updated
        id: Id,
        /// it will now be remembered for this long
        ttl: Duration,
    },
    /// TrackQuery::Lookup was successful
    LookupAns {
        /// this is the node to connect to (if one was found)
        adr: Option<SocketAddr>,
        /// reference to supply in the next request
        lookup_id: u32,
    }
}

impl TrackResp {
    pub fn is_lookup(&self) -> bool {
        match self {
            TrackResp::LookupAns{..} => true,
            _ => false,
        }
    }
    pub fn is_update(&self) -> bool {
        match self {
            TrackResp::UpdateSuccess{..} => true,
            _ => false,
        }
    }
}
