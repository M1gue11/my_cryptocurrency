use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerInfo {
    pub addr: String,
    pub direction: String,
    pub connection_state: String,
    pub handshake_state: String,
    pub connected_at: Option<String>,
    pub last_event_at: Option<String>,
    pub last_event: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeersListResponse {
    pub count: usize,
    pub peers: Vec<PeerInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerDisconnectParams {
    pub addr: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerDisconnectResponse {
    pub success: bool,
    pub message: Option<String>,
}
