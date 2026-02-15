import { useEffect, useState, useMemo } from "react";
import { Card, Button } from "../components";
import { rpcClient } from "../services";
import type { BlockInfo, ChainShowResponse } from "../types";

type SearchMode = "hash" | "height";

export function Blocks() {
  const [blocks, setBlocks] = useState<BlockInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedBlock, setSelectedBlock] = useState<BlockInfo | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchMode, setSearchMode] = useState<SearchMode>("hash");

  const fetchBlocks = async () => {
    try {
      setLoading(true);
      setError(null);
      const response: ChainShowResponse = await rpcClient.chainShow();
      setBlocks(response.blocks.reverse());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch blocks");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchBlocks();
  }, []);

  const filteredBlocks = useMemo(() => {
    if (!searchQuery.trim()) return blocks;

    const query = searchQuery.trim();

    if (searchMode === "height") {
      const heightNum = parseInt(query, 10);
      if (isNaN(heightNum)) return blocks;
      return blocks.filter((block) => block.height === heightNum);
    } else {
      const queryLower = query.toLowerCase();
      return blocks.filter((block) =>
        block.hash.toLowerCase().includes(queryLower)
      );
    }
  }, [blocks, searchQuery, searchMode]);

  if (loading && blocks.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading blocks...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <div className="text-red-400">{error}</div>
        <Button onClick={fetchBlocks}>Retry</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Blocks</h2>
        <Button onClick={fetchBlocks} variant="secondary">
          Refresh
        </Button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Block List */}
        <div className="lg:col-span-1">
          <Card title={`All Blocks (${blocks.length})`}>
            {/* Search Input with Toggle */}
            <div className="mb-4">
              <div className="flex gap-2">
                <input
                  type={searchMode === "height" ? "number" : "text"}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={
                    searchMode === "hash"
                      ? "Search by hash..."
                      : "Search by height..."
                  }
                  className="flex-1 bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500 placeholder-gray-500"
                />
                <button
                  onClick={() => {
                    setSearchMode(searchMode === "hash" ? "height" : "hash");
                    setSearchQuery("");
                  }}
                  className={`px-3 py-2 rounded text-xs font-medium transition-colors whitespace-nowrap ${
                    searchMode === "height"
                      ? "bg-blue-600 text-white"
                      : "bg-gray-700 text-gray-400 hover:bg-gray-600"
                  }`}
                  title="Toggle search by height"
                >
                  # Height
                </button>
              </div>
              {searchQuery && (
                <p className="text-xs text-gray-500 mt-1">
                  Showing {filteredBlocks.length} of {blocks.length} blocks
                </p>
              )}
            </div>

            <div className="space-y-2 max-h-130 overflow-y-auto custom-scrollbar">
              {filteredBlocks.map((block) => (
                <button
                  key={block.hash}
                  onClick={() => setSelectedBlock(block)}
                  className={`w-full text-left p-3 rounded transition-colors ${
                    selectedBlock?.hash === block.hash
                      ? "bg-blue-600"
                      : "bg-gray-700 hover:bg-gray-600"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-white">
                      #{block.height}
                    </span>
                    <span className="text-xs text-gray-400">
                      {block.transactions.length} txs
                    </span>
                  </div>
                  <div className="text-xs text-gray-400 font-mono truncate mt-1">
                    {block.hash}
                  </div>
                </button>
              ))}
              {filteredBlocks.length === 0 && searchQuery && (
                <p className="text-gray-400 text-center py-4">
                  No blocks match "{searchQuery}"
                </p>
              )}
              {blocks.length === 0 && (
                <p className="text-gray-400 text-center py-4">
                  No blocks found
                </p>
              )}
            </div>
          </Card>
        </div>

        {/* Block Details */}
        <div className="lg:col-span-2">
          {selectedBlock ? (
            <Card title={`Block #${selectedBlock.height}`}>
              <dl className="space-y-4">
                <div>
                  <dt className="text-gray-400 text-sm">Hash</dt>
                  <dd className="text-white font-mono text-sm break-all">
                    {selectedBlock.hash}
                  </dd>
                </div>
                <div>
                  <dt className="text-gray-400 text-sm">Previous Hash</dt>
                  <dd className="text-white font-mono text-sm break-all">
                    {selectedBlock.prev_hash}
                  </dd>
                </div>
                <div>
                  <dt className="text-gray-400 text-sm">Merkle Root</dt>
                  <dd className="text-white font-mono text-sm break-all">
                    {selectedBlock.merkle_root}
                  </dd>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <dt className="text-gray-400 text-sm">Nonce</dt>
                    <dd className="text-white font-mono">
                      {selectedBlock.nonce}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-gray-400 text-sm">Size</dt>
                    <dd className="text-white">
                      {selectedBlock.size_bytes} bytes
                    </dd>
                  </div>
                </div>
                <div>
                  <dt className="text-gray-400 text-sm">Timestamp</dt>
                  <dd className="text-white">{selectedBlock.timestamp}</dd>
                </div>

                {/* Transactions in block */}
                <div>
                  <dt className="text-gray-400 text-sm mb-2">
                    Transactions ({selectedBlock.transactions.length})
                  </dt>
                  <dd className="space-y-2 max-h-64 overflow-y-auto custom-scrollbar">
                    {selectedBlock.transactions.map((tx, idx) => (
                      <div key={idx} className="p-3 bg-gray-700 rounded text-sm">
                        <div className="font-mono text-xs text-gray-300 truncate">
                          {tx.id}
                        </div>
                        <div className="flex justify-between mt-1 text-xs">
                          <span className="text-gray-400">
                            {tx.inputs.length} inputs
                          </span>
                          <span className="text-gray-400">
                            {tx.outputs.length} outputs
                          </span>
                          <span className="text-green-400">
                            {tx.outputs.reduce((sum, o) => sum + o.value, 0)} units
                          </span>
                        </div>
                      </div>
                    ))}
                  </dd>
                </div>
              </dl>
            </Card>
          ) : (
            <Card>
              <div className="text-gray-400 text-center py-12">
                Select a block to view details
              </div>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
