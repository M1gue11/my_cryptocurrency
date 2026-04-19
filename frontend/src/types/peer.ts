// Peer Management Types
// Mirrors: project/src/daemon/types/peer.rs

export interface PeerInfo {
  addr: string;
  direction: "inbound" | "outbound";
  connection_state: "connected" | "disconnecting";
  handshake_state: "connecting" | "version_received" | "handshake_complete";
  connected_at: string | null;
  last_event_at: string | null;
  last_event: string | null;
}

export interface PeersListResponse {
  count: number;
  peers: PeerInfo[];
}

export interface PeerDisconnectResponse {
  success: boolean;
  message: string | null;
}
