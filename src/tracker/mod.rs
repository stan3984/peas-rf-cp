pub mod server;
pub mod api;

use common::id::Id;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
enum TrackQuery {
    Update {
        id: Id,
        adr: SocketAddr,
    },
    Lookup {
        id: Id,
        last_lookup: u32,
    }
}

enum TrackResp {
    UpdateSuccess {
        id: Id,
        ttl: Duration,
    },
    LookupAns {
        adr: Option<SocketAddr>,
        lookup_id: u32,
    }
}
