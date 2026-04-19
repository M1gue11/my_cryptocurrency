use std::net::SocketAddr;
use std::str::FromStr;

use crate::daemon::types::rpc::INVALID_PARAMS;
use crate::daemon::types::{
    PeerDisconnectParams, PeerDisconnectResponse, PeerInfo, PeersListResponse, RpcResponse,
};
use crate::network::{DisconnectPeerResult, disconnect_peer, list_connected_peers};

pub async fn handle_peers_list(id: Option<u64>) -> RpcResponse {
    let peers = list_connected_peers().await;
    let peer_list: Vec<PeerInfo> = peers
        .into_iter()
        .map(|peer| PeerInfo {
            addr: peer.addr.to_string(),
            direction: peer.direction.as_str().to_string(),
            connection_state: peer.connection_state.as_str().to_string(),
            handshake_state: peer.handshake_state.as_str().to_string(),
            connected_at: peer.connected_at.map(|v| v.to_string()),
            last_event_at: peer.last_event_at.map(|v| v.to_string()),
            last_event: peer.last_event,
        })
        .collect();

    let response = PeersListResponse {
        count: peer_list.len(),
        peers: peer_list,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_peer_disconnect(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: PeerDisconnectParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let addr = match SocketAddr::from_str(&params.addr) {
        Ok(addr) => addr,
        Err(_) => {
            return RpcResponse::error(
                id,
                INVALID_PARAMS,
                format!("Invalid peer address format: {}", params.addr),
            );
        }
    };

    let response = match disconnect_peer(addr).await {
        DisconnectPeerResult::Signaled => PeerDisconnectResponse {
            success: true,
            message: Some(format!("Disconnect signal sent to {}", addr)),
        },
        DisconnectPeerResult::AlreadyClosed => PeerDisconnectResponse {
            success: true,
            message: Some(format!("Peer {} was already closing", addr)),
        },
        DisconnectPeerResult::NotFound => PeerDisconnectResponse {
            success: false,
            message: Some(format!("Peer not found: {}", addr)),
        },
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}
