import { useEffect, useState } from 'react';
import { Card, StatCard, Button } from '../components';
import { rpcClient } from '../services';
import type { NodeStatusResponse, ChainStatusResponse, MempoolResponse } from '../types';

export function Dashboard() {
  const [nodeStatus, setNodeStatus] = useState<NodeStatusResponse | null>(null);
  const [chainStatus, setChainStatus] = useState<ChainStatusResponse | null>(null);
  const [mempool, setMempool] = useState<MempoolResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [mining, setMining] = useState(false);

  const fetchData = async () => {
    try {
      setLoading(true);
      setError(null);

      const [status, chain, mem] = await Promise.all([
        rpcClient.nodeStatus(),
        rpcClient.chainStatus(),
        rpcClient.nodeMempool(),
      ]);

      setNodeStatus(status);
      setChainStatus(chain);
      setMempool(mem);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect to daemon');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    // Refresh every 10 seconds
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, []);

  const handleMineBlock = async () => {
    try {
      setMining(true);
      const result = await rpcClient.mineBlock();
      if (result.success) {
        alert(`Block mined successfully! Hash: ${result.block_hash}`);
        fetchData(); // Refresh data
      } else {
        alert(`Mining failed: ${result.error}`);
      }
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Mining failed');
    } finally {
      setMining(false);
    }
  };

  if (loading && !nodeStatus) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Connecting to daemon...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <div className="text-red-400">⚠️ {error}</div>
        <Button onClick={fetchData}>Retry</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Dashboard</h2>
        <Button onClick={handleMineBlock} loading={mining}>
          ⛏️ Mine Block
        </Button>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon="📦"
          label="Block Height"
          value={nodeStatus?.block_height ?? 0}
        />
        <StatCard
          icon="🌐"
          label="Connected Peers"
          value={nodeStatus?.peers_connected ?? 0}
        />
        <StatCard
          icon="📋"
          label="Mempool Size"
          value={mempool?.count ?? 0}
        />
        <StatCard
          icon={chainStatus?.is_valid ? '✅' : '❌'}
          label="Chain Status"
          value={chainStatus?.is_valid ? 'Valid' : 'Invalid'}
        />
      </div>

      {/* Node Info */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title="Node Information">
          <dl className="space-y-3">
            <div className="flex justify-between">
              <dt className="text-gray-400">Version</dt>
              <dd className="text-white font-mono">{nodeStatus?.version}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Top Block Hash</dt>
              <dd className="text-white font-mono text-xs truncate max-w-xs">
                {nodeStatus?.top_block_hash}
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Last Block Date</dt>
              <dd className="text-white">{chainStatus?.last_block_date ?? 'N/A'}</dd>
            </div>
          </dl>
        </Card>

        <Card title="Mempool">
          {mempool && mempool.count > 0 ? (
            <div className="space-y-2">
              {mempool.transactions.slice(0, 5).map((entry, i) => (
                <div
                  key={i}
                  className="flex justify-between items-center p-2 bg-gray-700 rounded"
                >
                  <span className="font-mono text-xs truncate max-w-xs">
                    {entry.tx.id}
                  </span>
                  <span className="text-green-400">
                    {entry.tx.outputs.reduce((sum, o) => sum + o.value, 0)} units
                  </span>
                </div>
              ))}
              {mempool.count > 5 && (
                <p className="text-gray-400 text-sm text-center">
                  +{mempool.count - 5} more transactions
                </p>
              )}
            </div>
          ) : (
            <p className="text-gray-400 text-center py-4">No pending transactions</p>
          )}
        </Card>
      </div>
    </div>
  );
}
