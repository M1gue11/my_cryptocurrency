import { useEffect, useState } from "react";
import { Card, StatCard, Button } from "../components";
import { rpcClient } from "../services";
import type { MineBlockResponse, MiningInfoResponse } from "../types";

function formatElapsed(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return [h, m, s].map((v) => String(v).padStart(2, "0")).join(":");
}

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

// The API returns NaiveDateTime serialized from Utc::now().naive_utc() -- pure UTC.
// Append "Z" so the browser parses it as UTC, matching Date.now().
function parseApiDate(raw: string): Date {
  return new Date(`${raw.slice(0, 19)}Z`);
}

export function Mining() {
  const [miningInfo, setMiningInfo] = useState<MiningInfoResponse>(null);
  const [keepMining, setKeepMining] = useState(false);
  const [lastResult, setLastResult] = useState<MineBlockResponse | null>(null);
  const [mining, setMining] = useState(false);
  const [togglingKeepMining, setTogglingKeepMining] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());

  const fetchMiningInfo = async () => {
    try {
      const info = await rpcClient.mineInfo();
      setMiningInfo(info);
    } catch (err) {
      console.error("Failed to fetch mining info:", err);
    }
  };

  // Poll mining info every 5s
  useEffect(() => {
    fetchMiningInfo();
    const interval = setInterval(fetchMiningInfo, 5000);
    return () => clearInterval(interval);
  }, []);

  // Tick every second unconditionally -- cheap and avoids dependency issues
  // when the page loads with mining already in progress.
  useEffect(() => {
    const tick = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(tick);
  }, []);

  const isCurrentlyMining = miningInfo !== null;

  const elapsedSeconds =
    miningInfo ?
      Math.max(0, Math.floor((now - parseApiDate(miningInfo).getTime()) / 1000))
    : 0;

  const handleMineBlock = async () => {
    try {
      setMining(true);
      setError(null);
      setLastResult(null);
      const result = await rpcClient.mineBlock();
      setLastResult(result);
      if (!result.success) {
        setError(result.error ?? "Mining failed");
      }
      fetchMiningInfo();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Mining failed");
    } finally {
      setMining(false);
    }
  };

  const handleToggleKeepMining = async () => {
    try {
      setTogglingKeepMining(true);
      setError(null);
      const newValue = !keepMining;
      await rpcClient.keepMining(newValue);
      setKeepMining(newValue);
    } catch (err) {
      setError(
        err instanceof Error ?
          err.message
        : "Failed to update keep mining flag",
      );
    } finally {
      setTogglingKeepMining(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Mining</h2>
        <div className="flex gap-3">
          <Button
            onClick={handleToggleKeepMining}
            loading={togglingKeepMining}
            variant={keepMining ? "danger" : "secondary"}
          >
            {keepMining ? "Stop Auto-Mining" : "Start Auto-Mining"}
          </Button>
          <Button
            onClick={handleMineBlock}
            loading={mining}
            loadingText="Mining..."
          >
            Mine Block
          </Button>
        </div>
      </div>

      {/* Status Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div
          className={`rounded-lg border p-4 transition-colors ${
            isCurrentlyMining ?
              "bg-amber-950/40 border-amber-500/60"
            : "bg-gray-800 border-gray-700"
          }`}
        >
          <div className="flex items-center gap-3">
            <span
              className={`text-2xl ${isCurrentlyMining ? "animate-pulse" : ""}`}
            >
              {isCurrentlyMining ? "⛏" : "-"}
            </span>
            <div>
              <p className="text-sm text-gray-400">Status</p>
              <p
                className={`text-2xl font-bold ${
                  isCurrentlyMining ? "text-amber-400" : "text-white"
                }`}
              >
                {isCurrentlyMining ? "Mining" : "Idle"}
              </p>
            </div>
          </div>
        </div>

        <StatCard
          icon="~"
          label="Auto-Mining"
          value={keepMining ? "Enabled" : "Disabled"}
        />
        <StatCard
          icon="#"
          label="Last Difficulty"
          value={lastResult?.difficulty ?? "-"}
        />
      </div>

      {/* Active Mining Session */}
      {isCurrentlyMining && (
        <div className="rounded-lg border border-amber-500/60 bg-amber-950/40">
          <div className="px-4 py-3 border-b border-amber-500/40">
            <h3 className="text-lg font-semibold text-amber-400">
              Active Mining Session
            </h3>
          </div>
          <div className="p-4 space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Started at</span>
              <span className="text-white font-mono">
                {dateFormatter.format(parseApiDate(miningInfo!))}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-400">Elapsed</span>
              <span className="text-amber-400 font-mono text-xl font-bold tabular-nums">
                {formatElapsed(elapsedSeconds)}
              </span>
            </div>
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="p-4 bg-red-900/40 border border-red-700 rounded-lg text-red-400">
          {error}
        </div>
      )}

      {/* Last Block Result */}
      {lastResult && lastResult.success && (
        <Card title="Last Mined Block">
          <dl className="space-y-3">
            <div className="flex justify-between">
              <dt className="text-gray-400">Block Hash</dt>
              <dd className="text-white font-mono text-xs truncate max-w-xs">
                {lastResult.block_hash}
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Nonce</dt>
              <dd className="text-white font-mono">{lastResult.nonce}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Difficulty</dt>
              <dd className="text-white">{lastResult.difficulty}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Next Difficulty</dt>
              <dd className="text-white">{lastResult.next_difficulty}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-400">Transactions Included</dt>
              <dd className="text-white">{lastResult.transactions.length}</dd>
            </div>
          </dl>

          {lastResult.transactions.length > 0 && (
            <div className="mt-4 space-y-2">
              <h4 className="text-gray-400 text-sm">Transactions</h4>
              {lastResult.transactions.map((tx, i) => (
                <div
                  key={i}
                  className="flex justify-between items-center p-2 bg-gray-700 rounded"
                >
                  <span className="font-mono text-xs truncate max-w-xs">
                    {tx.id}
                  </span>
                  <span className="text-green-400 ml-4 whitespace-nowrap">
                    {tx.outputs.reduce((sum, o) => sum + o.value, 0)} units
                  </span>
                </div>
              ))}
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
