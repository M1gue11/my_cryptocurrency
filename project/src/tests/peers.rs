use std::net::SocketAddr;
#[cfg(test)]
mod tests {
    use crate::network::{
        DisconnectPeerResult,
        peer_manager::{PeerConnectionState, PeerDirection, PeerHandshakeState, PeerManager},
    };

    use super::*;

    #[tokio::test]
    async fn registers_and_lists_peer_with_initial_state() {
        let manager = PeerManager::new();
        let addr: SocketAddr = "127.0.0.1:6000".parse().unwrap();

        let (_connection_id, _rx) = manager.register_peer(addr, PeerDirection::Inbound).await;

        let peers = manager.list_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].addr, addr);
        assert_eq!(peers[0].direction, PeerDirection::Inbound);
        assert_eq!(peers[0].connection_state, PeerConnectionState::Connected);
        assert_eq!(peers[0].handshake_state, PeerHandshakeState::Connecting);
        assert!(peers[0].connected_at.is_some());
        assert!(peers[0].last_event_at.is_some());
        assert_eq!(
            peers[0].last_event.as_deref(),
            Some("Inbound connection accepted")
        );
    }

    #[tokio::test]
    async fn updates_handshake_state_for_active_peer() {
        let manager = PeerManager::new();
        let addr: SocketAddr = "127.0.0.1:6001".parse().unwrap();

        let (connection_id, _rx) = manager.register_peer(addr, PeerDirection::Outbound).await;
        manager
            .set_handshake_state(
                addr,
                connection_id,
                PeerHandshakeState::VersionReceived,
                "Received VERSION",
            )
            .await;
        manager
            .set_handshake_state(
                addr,
                connection_id,
                PeerHandshakeState::HandshakeComplete,
                "Received VERACK",
            )
            .await;

        let peers = manager.list_peers().await;
        assert_eq!(
            peers[0].handshake_state,
            PeerHandshakeState::HandshakeComplete
        );
        assert_eq!(peers[0].last_event.as_deref(), Some("Received VERACK"));
    }

    #[tokio::test]
    async fn disconnect_peer_signals_and_marks_state() {
        let manager = PeerManager::new();
        let addr: SocketAddr = "127.0.0.1:6002".parse().unwrap();

        let (_connection_id, mut rx) = manager.register_peer(addr, PeerDirection::Inbound).await;

        let result = manager.disconnect_peer(addr).await;
        assert_eq!(result, DisconnectPeerResult::Signaled);
        rx.changed().await.unwrap();
        assert!(*rx.borrow());

        let peers = manager.list_peers().await;
        assert_eq!(
            peers[0].connection_state,
            PeerConnectionState::Disconnecting
        );
        assert_eq!(
            peers[0].last_event.as_deref(),
            Some("Disconnect requested via RPC")
        );
    }

    #[tokio::test]
    async fn remove_peer_ignores_stale_connection_id() {
        let manager = PeerManager::new();
        let addr: SocketAddr = "127.0.0.1:6003".parse().unwrap();

        let (connection_id, _rx) = manager.register_peer(addr, PeerDirection::Inbound).await;
        let (replacement_id, _replacement_rx) =
            manager.register_peer(addr, PeerDirection::Outbound).await;

        manager.remove_peer(addr, connection_id).await;
        let peers = manager.list_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].direction, PeerDirection::Outbound);

        manager.remove_peer(addr, replacement_id).await;
        assert!(manager.list_peers().await.is_empty());
    }
}
