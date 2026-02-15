import { useEffect, useState } from 'react';
import { Card, StatCard, Button } from '../components';
import { rpcClient } from '../services';
import type { NodeStatusResponse } from '../types';

export function Network() {
  const [nodeStatus, setNodeStatus] = useState<NodeStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = async () => {
    try {
      setLoading(true);
      setError(null);
      const status = await rpcClient.nodeStatus();
      setNodeStatus(status);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch network status');
    } finally {
      setLoading(false);
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
        <div className="text-center py-8">
          <p className="text-gray-400 mb-4">
            Peer IP addresses are not yet available in the API.
          </p>
          <p className="text-gray-500 text-sm">
            To add this feature, implement a <code className="bg-gray-700 px-1 rounded">node_peers</code> method
            in the daemon that returns the list of connected peer addresses.
          </p>
        </div>
      </Card>
    </div>
  );
}
