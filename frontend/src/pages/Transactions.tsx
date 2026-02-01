import { useEffect, useState } from 'react';
import { Card, Button } from '../components';
import { rpcClient } from '../services';
import type { MempoolResponse, TransactionViewResponse } from '../types';

export function Transactions() {
  const [mempool, setMempool] = useState<MempoolResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Search state
  const [searchId, setSearchId] = useState('');
  const [searchResult, setSearchResult] = useState<TransactionViewResponse | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);

  const fetchMempool = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await rpcClient.nodeMempool();
      setMempool(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch mempool');
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchId.trim()) return;

    try {
      setSearching(true);
      setSearchError(null);
      setSearchResult(null);
      const result = await rpcClient.transactionView(searchId.trim());
      setSearchResult(result);
    } catch (err) {
      setSearchError(err instanceof Error ? err.message : 'Transaction not found');
    } finally {
      setSearching(false);
    }
  };

  const clearSearch = () => {
    setSearchId('');
    setSearchResult(null);
    setSearchError(null);
  };

  useEffect(() => {
    fetchMempool();
    const interval = setInterval(fetchMempool, 10000);
    return () => clearInterval(interval);
  }, []);

  const clearMempool = async () => {
    try {
      await rpcClient.nodeClearMempool();
      fetchMempool();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to clear mempool');
    }
  };

  if (loading && !mempool) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading transactions...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Transactions</h2>
        <div className="flex gap-2">
          <Button onClick={fetchMempool} variant="secondary">
            Refresh
          </Button>
          <Button onClick={clearMempool} variant="danger">
            Clear Mempool
          </Button>
        </div>
      </div>

      {error && (
        <div className="bg-red-900/50 border border-red-500 rounded p-3 text-red-200">
          {error}
        </div>
      )}

      {/* Search */}
      <Card title="Search Transaction">
        <form onSubmit={handleSearch} className="flex gap-2">
          <input
            type="text"
            value={searchId}
            onChange={(e) => setSearchId(e.target.value)}
            className="flex-1 bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500 font-mono text-sm"
            placeholder="Enter transaction ID (hex)"
          />
          <Button type="submit" loading={searching}>
            Search
          </Button>
          {(searchResult || searchError) && (
            <Button type="button" variant="secondary" onClick={clearSearch}>
              Clear
            </Button>
          )}
        </form>

        {searchError && (
          <p className="text-red-400 mt-3">{searchError}</p>
        )}

        {searchResult && (
          <div className="mt-4">
            <TransactionDetail tx={searchResult} />
          </div>
        )}
      </Card>

      {/* Mempool */}
      <Card title={`Mempool (${mempool?.count ?? 0} pending)`}>
        {mempool && mempool.count > 0 ? (
          <div className="space-y-3">
            {mempool.transactions.map((entry, idx) => (
              <div
                key={idx}
                className="p-4 bg-gray-700 rounded"
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="font-mono text-sm text-gray-300 truncate max-w-md">
                    {entry.tx.id}
                  </span>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      setSearchId(entry.tx.id);
                      setSearchResult(entry.tx);
                      setSearchError(null);
                    }}
                  >
                    View
                  </Button>
                </div>
                <div className="grid grid-cols-4 gap-4 text-sm">
                  <div>
                    <span className="text-gray-400">Inputs:</span>{' '}
                    <span className="text-white">{entry.tx.inputs.length}</span>
                  </div>
                  <div>
                    <span className="text-gray-400">Outputs:</span>{' '}
                    <span className="text-white">{entry.tx.outputs.length}</span>
                  </div>
                  <div>
                    <span className="text-gray-400">Value:</span>{' '}
                    <span className="text-green-400">
                      {entry.tx.outputs.reduce((sum, o) => sum + o.value, 0)} units
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-400">Size:</span>{' '}
                    <span className="text-white">{entry.tx.size} bytes</span>
                  </div>
                </div>
                {entry.tx.message && (
                  <div className="mt-2 text-sm">
                    <span className="text-gray-400">Message:</span>{' '}
                    <span className="text-white">{entry.tx.message}</span>
                  </div>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p className="text-gray-400 text-center py-8">No pending transactions in mempool</p>
        )}
      </Card>
    </div>
  );
}

function TransactionDetail({ tx }: { tx: TransactionViewResponse }) {
  return (
    <div className="bg-gray-700 rounded p-4 space-y-4">
      <div>
        <span className="text-gray-400 text-sm">Transaction ID</span>
        <p className="font-mono text-sm text-white break-all">{tx.id}</p>
      </div>

      <div className="grid grid-cols-3 gap-4 text-sm">
        <div>
          <span className="text-gray-400">Date:</span>{' '}
          <span className="text-white">{tx.date}</span>
        </div>
        <div>
          <span className="text-gray-400">Size:</span>{' '}
          <span className="text-white">{tx.size} bytes</span>
        </div>
        <div>
          <span className="text-gray-400">Type:</span>{' '}
          <span className={tx.is_coinbase ? 'text-yellow-400' : 'text-white'}>
            {tx.is_coinbase ? 'Coinbase' : 'Regular'}
          </span>
        </div>
      </div>

      {tx.message && (
        <div>
          <span className="text-gray-400 text-sm">Message:</span>
          <p className="text-white">{tx.message}</p>
        </div>
      )}

      {/* Inputs */}
      <div>
        <span className="text-gray-400 text-sm">Inputs ({tx.inputs.length})</span>
        <div className="mt-2 space-y-2">
          {tx.inputs.length > 0 ? (
            tx.inputs.map((input, idx) => (
              <div key={idx} className="bg-gray-800 p-2 rounded text-xs">
                <div className="flex justify-between">
                  <span className="text-gray-400">From TX:</span>
                  <span className="font-mono text-gray-300 truncate max-w-xs">
                    {input.prev_tx_id}
                  </span>
                </div>
                <div className="flex justify-between mt-1">
                  <span className="text-gray-400">Output Index:</span>
                  <span className="text-white">{input.output_index}</span>
                </div>
              </div>
            ))
          ) : (
            <p className="text-gray-500 text-xs">No inputs (coinbase)</p>
          )}
        </div>
      </div>

      {/* Outputs */}
      <div>
        <span className="text-gray-400 text-sm">Outputs ({tx.outputs.length})</span>
        <div className="mt-2 space-y-2">
          {tx.outputs.map((output, idx) => (
            <div key={idx} className="bg-gray-800 p-2 rounded text-xs flex justify-between items-center">
              <span className="font-mono text-gray-300 truncate max-w-xs">
                {output.address}
              </span>
              <span className="text-green-400 font-bold">
                {output.value} units
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
