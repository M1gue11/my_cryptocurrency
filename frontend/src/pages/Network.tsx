import { useEffect, useState } from 'react';
import { Card, StatCard, Button } from '../components';
import { rpcClient } from '../services';
import type { NodeStatusResponse, PeerInfo } from '../types';

export function Network() {
  const [nodeStatus, setNodeStatus] = useState<NodeStatusResponse | null>(null);
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [disconnectingPeer, setDisconnectingPeer] = useState<string | null>(null);

  const formatDateTime = (value: string | null) => {
    if (!value) return 'Unknown';
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return value;
    return parsed.toLocaleString();
  };

  const getDirectionBadge = (direction: PeerInfo['direction']) => {
    if (direction === 'outbound') {
      return 'bg-blue-500/15 text-blue-300 border border-blue-500/30';
    }
    return 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30';
  };

  const getHandshakeBadge = (state: PeerInfo['handshake_state']) => {
    switch (state) {
      case 'handshake_complete':
        return 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30';
      case 'version_received':
        return 'bg-amber-500/15 text-amber-300 border border-amber-500/30';
      default:
        return 'bg-gray-600/30 text-gray-300 border border-gray-600/40';
    }
  };

  const getConnectionBadge = (state: PeerInfo['connection_state']) => {
    if (state === 'disconnecting') {
      return 'bg-red-500/15 text-red-300 border border-red-500/30';
    }
    return 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30';
  };

  const fetchStatus = async () => {
    try {
      setLoading(true);
      setError(null);
      const [status, peersResponse] = await Promise.all([
        rpcClient.nodeStatus(),
        rpcClient.peersList(),
      ]);
      setNodeStatus(status);
      setPeers(peersResponse.peers);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch network status');
    } finally {
      setLoading(false);
    }
  };

  const handleDisconnectPeer = async (addr: string) => {
    try {
      setDisconnectingPeer(addr);
      setActionError(null);
      setActionMessage(null);
      const response = await rpcClient.peerDisconnect(addr);
      if (!response.success) {
        setActionError(response.message ?? `Failed to disconnect peer ${addr}`);
        return;
      }
      setActionMessage(response.message ?? `Disconnect requested for ${addr}`);
      await fetchStatus();
    } catch (err) {
      setActionError(err instanceof Error ? err.message : `Failed to disconnect peer ${addr}`);
    } finally {
      setDisconnectingPeer(null);
    }
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading && !nodeStatus) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading network status...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <div className="text-red-400">{error}</div>
        <Button onClick={fetchStatus}>Retry</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Network</h2>
        <Button onClick={fetchStatus} variant="secondary" loading={loading}>
          Refresh
        </Button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <StatCard
          icon="🌐"
          label="Connected Peers"
          value={nodeStatus?.peers_connected ?? 0}
        />
        <StatCard
          icon="📦"
          label="Block Height"
          value={nodeStatus?.block_height ?? 0}
        />
        <StatCard
          icon="🔧"
          label="Version"
          value={nodeStatus?.version ?? 'Unknown'}
        />
      </div>

      {/* Node Info */}
      <Card title="Node Information">
        <dl className="space-y-4">
          <div>
            <dt className="text-gray-400 text-sm">Top Block Hash</dt>
            <dd className="text-white font-mono text-sm break-all">
              {nodeStatus?.top_block_hash}
            </dd>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-gray-400 text-sm">P2P Port</dt>
              <dd className="text-white font-mono">6000</dd>
            </div>
            <div>
              <dt className="text-gray-400 text-sm">HTTP Port</dt>
              <dd className="text-white font-mono">7001</dd>
            </div>
          </div>
        </dl>
      </Card>

      {/* API Limitation Notice */}
      <Card title="Peer Details">
        <div className="space-y-4">
          {actionError && (
            <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-300">
              {actionError}
            </div>
          )}

          {actionMessage && (
            <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-300">
              {actionMessage}
            </div>
          )}

          {peers.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-gray-300 mb-2">No active peers connected right now.</p>
              <p className="text-gray-500 text-sm">
                When the node accepts or opens P2P connections, they will appear here with handshake
                status and recent activity.
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              {peers.map((peer) => (
                <div
                  key={peer.addr}
                  className="rounded-lg border border-gray-700 bg-gray-900/50 p-4"
                >
                  <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                    <div className="space-y-3">
                      <div>
                        <div className="text-xs uppercase tracking-wide text-gray-400">Peer</div>
                        <div className="break-all font-mono text-sm text-white">{peer.addr}</div>
                      </div>

                      <div className="flex flex-wrap gap-2">
                        <span
                          className={`rounded-full px-2.5 py-1 text-xs font-medium ${getDirectionBadge(peer.direction)}`}
                        >
                          {peer.direction}
                        </span>
                        <span
                          className={`rounded-full px-2.5 py-1 text-xs font-medium ${getConnectionBadge(peer.connection_state)}`}
                        >
                          {peer.connection_state}
                        </span>
                        <span
                          className={`rounded-full px-2.5 py-1 text-xs font-medium ${getHandshakeBadge(peer.handshake_state)}`}
                        >
                          {peer.handshake_state}
                        </span>
                      </div>
                    </div>

                    <Button
                      variant="danger"
                      size="sm"
                      loading={disconnectingPeer === peer.addr}
                      loadingText="Disconnecting..."
                      onClick={() => handleDisconnectPeer(peer.addr)}
                      disabled={peer.connection_state === 'disconnecting'}
                    >
                      Disconnect
                    </Button>
                  </div>

                  <div className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-3">
                    <div>
                      <div className="text-xs uppercase tracking-wide text-gray-400">Connected Since</div>
                      <div className="text-sm text-gray-200">{formatDateTime(peer.connected_at)}</div>
                    </div>
                    <div>
                      <div className="text-xs uppercase tracking-wide text-gray-400">Last Event At</div>
                      <div className="text-sm text-gray-200">{formatDateTime(peer.last_event_at)}</div>
                    </div>
                    <div>
                      <div className="text-xs uppercase tracking-wide text-gray-400">Last Event</div>
                      <div className="text-sm text-gray-200">{peer.last_event ?? 'No events recorded'}</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
