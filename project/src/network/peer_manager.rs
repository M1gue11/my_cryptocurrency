use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::NaiveDateTime;
use once_cell::sync::Lazy;
use tokio::sync::{RwLock, watch};

use crate::utils::get_current_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerDirection {
    Inbound,
    Outbound,
}

impl PeerDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            PeerDirection::Inbound => "inbound",
            PeerDirection::Outbound => "outbound",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionState {
    Connected,
    Disconnecting,
}

impl PeerConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PeerConnectionState::Connected => "connected",
            PeerConnectionState::Disconnecting => "disconnecting",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerHandshakeState {
    Connecting,
    VersionReceived,
    HandshakeComplete,
}

impl PeerHandshakeState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PeerHandshakeState::Connecting => "connecting",
            PeerHandshakeState::VersionReceived => "version_received",
            PeerHandshakeState::HandshakeComplete => "handshake_complete",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeerSnapshot {
    pub addr: SocketAddr,
    pub direction: PeerDirection,
    pub connection_state: PeerConnectionState,
    pub handshake_state: PeerHandshakeState,
    pub connected_at: Option<NaiveDateTime>,
    pub last_event_at: Option<NaiveDateTime>,
    pub last_event: Option<String>,
}

struct PeerEntry {
    connection_id: u64,
    info: PeerSnapshot,
    disconnect_tx: watch::Sender<bool>,
}

pub struct PeerManager {
    peers: Arc<RwLock<HashMap<SocketAddr, PeerEntry>>>,
    next_connection_id: AtomicU64,
}

pub static PEER_MANAGER: Lazy<PeerManager> = Lazy::new(PeerManager::new);

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            next_connection_id: AtomicU64::new(1),
        }
    }

    pub async fn register_peer(
        &self,
        addr: SocketAddr,
        direction: PeerDirection,
    ) -> (u64, watch::Receiver<bool>) {
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let now = get_current_timestamp();
        let (disconnect_tx, disconnect_rx) = watch::channel(false);
        let initial_event = match direction {
            PeerDirection::Inbound => "Inbound connection accepted",
            PeerDirection::Outbound => "Outbound connection established",
        };

        let entry = PeerEntry {
            connection_id,
            info: PeerSnapshot {
                addr,
                direction,
                connection_state: PeerConnectionState::Connected,
                handshake_state: PeerHandshakeState::Connecting,
                connected_at: Some(now),
                last_event_at: Some(now),
                last_event: Some(initial_event.to_string()),
            },
            disconnect_tx,
        };

        let mut peers = self.peers.write().await;
        if let Some(previous) = peers.insert(addr, entry) {
            let _ = previous.disconnect_tx.send(true);
        }

        (connection_id, disconnect_rx)
    }

    pub async fn set_handshake_state(
        &self,
        addr: SocketAddr,
        connection_id: u64,
        handshake_state: PeerHandshakeState,
        last_event: impl Into<String>,
    ) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(&addr) {
            if peer.connection_id != connection_id {
                return;
            }
            peer.info.handshake_state = handshake_state;
            peer.info.last_event_at = Some(get_current_timestamp());
            peer.info.last_event = Some(last_event.into());
        }
    }

    pub async fn update_last_event(
        &self,
        addr: SocketAddr,
        connection_id: u64,
        last_event: impl Into<String>,
    ) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(&addr) {
            if peer.connection_id != connection_id {
                return;
            }
            peer.info.last_event_at = Some(get_current_timestamp());
            peer.info.last_event = Some(last_event.into());
        }
    }

    pub async fn mark_disconnecting(
        &self,
        addr: SocketAddr,
        connection_id: u64,
        last_event: impl Into<String>,
    ) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(&addr) {
            if peer.connection_id != connection_id {
                return;
            }
            peer.info.connection_state = PeerConnectionState::Disconnecting;
            peer.info.last_event_at = Some(get_current_timestamp());
            peer.info.last_event = Some(last_event.into());
        }
    }

    pub async fn disconnect_peer(&self, addr: SocketAddr) -> DisconnectPeerResult {
        let mut peers = self.peers.write().await;
        let Some(peer) = peers.get_mut(&addr) else {
            return DisconnectPeerResult::NotFound;
        };

        peer.info.connection_state = PeerConnectionState::Disconnecting;
        peer.info.last_event_at = Some(get_current_timestamp());
        peer.info.last_event = Some("Disconnect requested via RPC".to_string());

        match peer.disconnect_tx.send(true) {
            Ok(_) => DisconnectPeerResult::Signaled,
            Err(_) => {
                peers.remove(&addr);
                DisconnectPeerResult::AlreadyClosed
            }
        }
    }

    pub async fn remove_peer(&self, addr: SocketAddr, connection_id: u64) {
        let mut peers = self.peers.write().await;
        let should_remove = peers
            .get(&addr)
            .map(|peer| peer.connection_id == connection_id)
            .unwrap_or(false);
        if should_remove {
            peers.remove(&addr);
        }
    }

    pub async fn get_peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    pub async fn list_peers(&self) -> Vec<PeerSnapshot> {
        let peers = self.peers.read().await;
        let mut snapshots: Vec<_> = peers.values().map(|peer| peer.info.clone()).collect();
        snapshots.sort_by_key(|peer| peer.addr.to_string());
        snapshots
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectPeerResult {
    Signaled,
    AlreadyClosed,
    NotFound,
}

pub async fn get_peer_count() -> usize {
    PEER_MANAGER.get_peer_count().await
}

pub async fn list_connected_peers() -> Vec<PeerSnapshot> {
    PEER_MANAGER.list_peers().await
}

pub async fn disconnect_peer(addr: SocketAddr) -> DisconnectPeerResult {
    PEER_MANAGER.disconnect_peer(addr).await
}
