import { useEffect, useState, useMemo } from "react";
import { Card, Button } from "../components";
import { rpcClient } from "../services";
import type { LogEntry, LogCategory, LogLevel } from "../types";

const CATEGORIES: LogCategory[] = ["Core", "P2P", "RPC"];
const LEVELS: LogLevel[] = ["Info", "Warning", "Error"];

const LEVEL_COLORS: Record<LogLevel, string> = {
  Info: "text-blue-400",
  Warning: "text-yellow-400",
  Error: "text-red-400",
};

const CATEGORY_COLORS: Record<LogCategory, string> = {
  Core: "bg-purple-600",
  P2P: "bg-green-600",
  RPC: "bg-blue-600",
};

export function Logs() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedLevel, setSelectedLevel] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [limit, setLimit] = useState(100);

  const fetchLogs = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await rpcClient.getLogs(
        selectedCategory?.toLowerCase() ?? undefined,
        selectedLevel?.toLowerCase() ?? undefined,
        limit,
      );
      setLogs(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch logs");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchLogs();
  }, [selectedCategory, selectedLevel, limit]);

  const filteredLogs = useMemo(() => {
    if (!searchQuery.trim()) return logs;
    const query = searchQuery.toLowerCase();
    return logs.filter((log) => log.message.toLowerCase().includes(query));
  }, [logs, searchQuery]);

  // Count by level for summary
  const levelCounts = useMemo(() => {
    const counts = { Info: 0, Warning: 0, Error: 0 };
    for (const log of logs) {
      counts[log.level]++;
    }
    return counts;
  }, [logs]);

  if (loading && logs.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading logs...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <div className="text-red-400">{error}</div>
        <Button onClick={fetchLogs}>Retry</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Logs</h2>
        <Button onClick={fetchLogs} variant="secondary">
          Refresh
        </Button>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-sm text-gray-400">Info</div>
          <div className="text-2xl font-bold text-blue-400">
            {levelCounts.Info}
          </div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-sm text-gray-400">Warning</div>
          <div className="text-2xl font-bold text-yellow-400">
            {levelCounts.Warning}
          </div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-sm text-gray-400">Error</div>
          <div className="text-2xl font-bold text-red-400">
            {levelCounts.Error}
          </div>
        </div>
      </div>

      {/* Filters */}
      <Card>
        <div className="flex flex-wrap items-center gap-4">
          {/* Category Filter */}
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-400">Category:</span>
            <button
              onClick={() => setSelectedCategory(null)}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                selectedCategory === null
                  ? "bg-blue-600 text-white"
                  : "bg-gray-700 text-gray-400 hover:bg-gray-600"
              }`}
            >
              All
            </button>
            {CATEGORIES.map((cat) => (
              <button
                key={cat}
                onClick={() => setSelectedCategory(cat)}
                className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                  selectedCategory === cat
                    ? `${CATEGORY_COLORS[cat]} text-white`
                    : "bg-gray-700 text-gray-400 hover:bg-gray-600"
                }`}
              >
                {cat}
              </button>
            ))}
          </div>

          {/* Level Filter */}
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-400">Level:</span>
            <button
              onClick={() => setSelectedLevel(null)}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                selectedLevel === null
                  ? "bg-blue-600 text-white"
                  : "bg-gray-700 text-gray-400 hover:bg-gray-600"
              }`}
            >
              All
            </button>
            {LEVELS.map((lvl) => (
              <button
                key={lvl}
                onClick={() => setSelectedLevel(lvl)}
                className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                  selectedLevel === lvl
                    ? "bg-blue-600 text-white"
                    : "bg-gray-700 text-gray-400 hover:bg-gray-600"
                }`}
              >
                {lvl}
              </button>
            ))}
          </div>

          {/* Limit Selector */}
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-400">Limit:</span>
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-white text-sm focus:outline-none focus:border-blue-500"
            >
              <option value={50}>50</option>
              <option value={100}>100</option>
              <option value={200}>200</option>
              <option value={500}>500</option>
            </select>
          </div>

          {/* Search */}
          <div className="flex-1 min-w-[200px]">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search in messages..."
              className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-1 text-white text-sm focus:outline-none focus:border-blue-500 placeholder-gray-500"
            />
          </div>
        </div>
      </Card>

      {/* Log Entries */}
      <Card title={`Log Entries (${filteredLogs.length})`}>
        <div className="space-y-1 max-h-[600px] overflow-y-auto custom-scrollbar">
          {filteredLogs.length === 0 ? (
            <p className="text-gray-400 text-center py-8">
              No logs found matching the current filters
            </p>
          ) : (
            filteredLogs.map((entry, idx) => (
              <div
                key={idx}
                className="flex items-start gap-3 px-3 py-2 rounded hover:bg-gray-700/50 font-mono text-sm"
              >
                {/* Timestamp */}
                <span className="text-gray-500 whitespace-nowrap shrink-0">
                  {entry.timestamp}
                </span>

                {/* Level Badge */}
                <span
                  className={`font-bold whitespace-nowrap w-14 shrink-0 ${LEVEL_COLORS[entry.level]}`}
                >
                  {entry.level === "Warning" ? "WARN" : entry.level.toUpperCase()}
                </span>

                {/* Category Badge */}
                <span
                  className={`${CATEGORY_COLORS[entry.category]} text-white text-xs px-2 py-0.5 rounded whitespace-nowrap shrink-0`}
                >
                  {entry.category}
                </span>

                {/* Message */}
                <span className="text-gray-200 break-all">
                  {entry.message}
                </span>
              </div>
            ))
          )}
        </div>
      </Card>
    </div>
  );
}
