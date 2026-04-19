import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import {
  ConsoleButton,
  ConsoleEmpty,
  ConsolePageHeader,
  ConsolePanel,
  ConsolePill,
  ConsoleRow,
  ConsoleStat,
  ConsoleStatStrip,
  shortHash,
} from '../components';
import { rpcClient } from '../services';
import type { MineBlockResponse, MiningInfoResponse } from '../types';

function formatElapsed(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainingSeconds = seconds % 60;
  return [hours, minutes, remainingSeconds]
    .map((value) => String(value).padStart(2, '0'))
    .join(':');
}

function parseApiDate(raw: string): Date {
  return new Date(`${raw.slice(0, 19)}Z`);
}

function compactTarget(value: string, edge = 10): string {
  if (!value) return '-';
  if (value.length <= edge * 2 + 5) return value;
  return `${value.slice(0, edge + 2)}...${value.slice(-edge)}`;
}

function TargetValue({
  value,
  edge = 10,
  className = '',
}: {
  value?: string | null;
  edge?: number;
  className?: string;
}) {
  if (!value) {
    return <span>-</span>;
  }

  return (
    <span
      className={`block min-w-0 max-w-full overflow-hidden text-ellipsis whitespace-nowrap ${className}`.trim()}
      title={value}
    >
      {compactTarget(value, edge)}
    </span>
  );
}

export function Mining() {
  const [miningInfo, setMiningInfo] = useState<MiningInfoResponse>({
    keep_mining_enabled: false,
    is_currently_mining: false,
    started_at: null,
    last_mined_block: null,
  });
  const [lastResult, setLastResult] = useState<MineBlockResponse | null>(null);
  const [mining, setMining] = useState(false);
  const [togglingKeepMining, setTogglingKeepMining] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const previousStartedAtRef = useRef<string | null>(null);

  const loadMiningInfo = useCallback(async () => {
    try {
      const info = await rpcClient.mineInfo();
      setMiningInfo(info);
    } catch (nextError) {
      setError(
        nextError instanceof Error ? nextError.message : 'Failed to fetch mining info',
      );
    }
  }, []);

  useEffect(() => {
    void loadMiningInfo();
    const interval = setInterval(() => {
      void loadMiningInfo();
    }, 5000);
    return () => clearInterval(interval);
  }, [loadMiningInfo]);

  useEffect(() => {
    const tick = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(tick);
  }, []);

  useEffect(() => {
    const previousStartedAt = previousStartedAtRef.current;
    const currentStartedAt = miningInfo.started_at ?? null;

    const syncLastMinedBlock = async () => {
      try {
        const block = await rpcClient.lastMinedBlock();
        if (block?.success) {
          setLastResult((current) => {
            if (current?.block_hash === block.block_hash) {
              return current;
            }
            return block;
          });
        }
      } catch {
        // Keep the previous result visible if this secondary request fails.
      }
    };

    if (previousStartedAt !== null && previousStartedAt !== currentStartedAt) {
      void syncLastMinedBlock();
    }

    previousStartedAtRef.current = currentStartedAt;
  }, [miningInfo.started_at]);

  const isCurrentlyMining = miningInfo.is_currently_mining;
  const keepMining = miningInfo.keep_mining_enabled;

  const elapsedSeconds =
    miningInfo.started_at
      ? Math.max(
          0,
          Math.floor((now - parseApiDate(miningInfo.started_at).getTime()) / 1000),
        )
      : 0;

  const statusTone = isCurrentlyMining ? 'accent' : 'neutral';
  const derivedStatusLabel = isCurrentlyMining ? 'searching nonce' : 'miner idle';

  const handleMineBlock = async () => {
    try {
      if (keepMining) {
        setError('Auto-mining is active. Disable it before mining manually.');
        return;
      }

      setMining(true);
      setError(null);
      const result = await rpcClient.mineBlock();
      setLastResult(result);

      if (!result.success) {
        setError(result.error ?? 'Mining failed');
      }

      await loadMiningInfo();
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : 'Mining failed');
    } finally {
      setMining(false);
    }
  };

  const handleToggleKeepMining = async () => {
    try {
      setTogglingKeepMining(true);
      setError(null);
      await rpcClient.keepMining(!keepMining);
      await loadMiningInfo();
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : 'Failed to update keep mining flag',
      );
    } finally {
      setTogglingKeepMining(false);
    }
  };

  const latestTransactions = useMemo(
    () => lastResult?.transactions ?? [],
    [lastResult],
  );

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow="mine_info . mine_block . mine_keep_mining"
        title="Mining Center"
        actions={
          <>
            <ConsolePill tone={keepMining ? 'accent' : 'neutral'}>
              keep_mining {keepMining ? 'on' : 'off'}
            </ConsolePill>
            <ConsoleButton
              onClick={handleToggleKeepMining}
              loading={togglingKeepMining}
            >
              toggle keep_on
            </ConsoleButton>
            <ConsoleButton
              tone="primary"
              onClick={handleMineBlock}
              loading={mining}
              loadingText="mining..."
              disabled={keepMining}
            >
              mine block now
            </ConsoleButton>
          </>
        }
      />

      {error ? (
        <div className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
          {error}
        </div>
      ) : null}

      <ConsoleStatStrip columns={4}>
        <ConsoleStat
          label="status"
          value={isCurrentlyMining ? 'mining' : 'idle'}
          subtitle={derivedStatusLabel}
          tone={statusTone}
        />
        <ConsoleStat
          label="keep mining"
          value={keepMining ? 'enabled' : 'disabled'}
          subtitle="daemon flag"
          tone={keepMining ? 'accent' : 'neutral'}
        />
        <ConsoleStat
          label="started"
          value={miningInfo.started_at ? formatElapsed(elapsedSeconds) : '-'}
          subtitle={miningInfo.started_at ?? 'no active session'}
        />
        <ConsoleStat
          label="last target"
          value={<TargetValue value={lastResult?.target} edge={10} />}
          subtitle={
            lastResult?.next_target ? (
              <TargetValue
                value={lastResult.next_target}
                edge={12}
                className="text-[0.64rem] text-[var(--crm-dim)]"
              />
            ) : (
              'awaiting mined block'
            )
          }
        />
      </ConsoleStatStrip>

      <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-5">
        <div className="grid gap-6 lg:grid-cols-[1fr_2fr]">
          <div className="flex items-center gap-5">
            <div className="relative h-24 w-24 shrink-0">
              <div className="absolute inset-0 rounded-full border border-[var(--crm-border)]" />
              {isCurrentlyMining ? (
                <>
                  <div
                    className="absolute inset-0 rounded-full border border-[var(--crm-accent)] border-r-transparent border-b-transparent"
                    style={{ animation: 'crm-spin 1.6s linear infinite' }}
                  />
                  <div
                    className="absolute inset-[10px] rounded-full border border-[var(--crm-accent-dim)] border-l-transparent border-t-transparent"
                    style={{ animation: 'crm-spin 2.4s linear infinite reverse' }}
                  />
                </>
              ) : null}
              <div className="absolute inset-0 grid place-items-center crm-mono text-xs tracking-[0.12em] text-[var(--crm-accent)]">
                {isCurrentlyMining ? 'HASH' : 'IDLE'}
              </div>
            </div>

            <div>
              <div className="crm-field-label">{derivedStatusLabel}</div>
              <div className="crm-mono text-3xl tracking-[-0.05em] text-[var(--crm-accent)]">
                {miningInfo.started_at ? formatElapsed(elapsedSeconds) : '--:--:--'}
              </div>
              <div className="mt-1 text-sm text-[var(--crm-dim)]">
                {miningInfo.started_at
                  ? `started at ${parseApiDate(miningInfo.started_at).toLocaleTimeString()}`
                  : 'No active mining session'}
              </div>
            </div>
          </div>

          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <Metric
              label="target"
              value={<TargetValue value={lastResult?.target} edge={10} />}
              subtitle="current"
            />
            <Metric
              label="difficulty"
              value={lastResult?.next_difficulty ?? '-'}
              subtitle="next difficulty"
            />
            <Metric
              label="last block"
              value={lastResult?.block_hash ? shortHash(lastResult.block_hash, 10) : '-'}
              subtitle="accepted block"
            />
            <Metric
              label="nonce"
              value={lastResult?.nonce?.toLocaleString() ?? '-'}
              subtitle="most recent result"
            />
          </div>
        </div>
      </div>

      <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-4">
        <div className="mb-3 flex flex-wrap items-center gap-3">
          <div className="crm-field-label">mining progress . conceptual</div>
          <div className="ml-auto crm-mono text-xs text-[var(--crm-dim)]">
            {isCurrentlyMining
              ? 'trying nonces... waiting for valid target'
              : 'miner idle - toggle keep_mining or run a manual attempt'}
          </div>
        </div>
        <div className="relative h-2 overflow-hidden rounded-full bg-[var(--crm-panel-2)]">
          {isCurrentlyMining ? (
            <div
              className="absolute inset-y-0 w-[40%] bg-[linear-gradient(90deg,transparent,var(--crm-accent),transparent)]"
              style={{ animation: 'crm-slide 1.8s ease-in-out infinite' }}
            />
          ) : null}
        </div>
        <div className="mt-3 flex flex-wrap justify-between gap-2 crm-mono text-[10px] text-[var(--crm-dim)]">
          <span>candidate block built</span>
          <span>transactions selected</span>
          <span className={isCurrentlyMining ? 'text-[var(--crm-accent)]' : ''}>
            hashing header
          </span>
          <span>valid nonce found</span>
          <span>block submitted</span>
        </div>
      </div>

      <div className="grid gap-3 xl:grid-cols-[1.25fr_1fr]">
        <ConsolePanel
          title="last mined block"
          subtitle="mine_last_block"
          icon="#"
          chip={
            lastResult?.success ? (
              <ConsolePill tone="good">accepted</ConsolePill>
            ) : (
              <ConsolePill tone="neutral">no result</ConsolePill>
            )
          }
        >
          {lastResult?.success ? (
            <>
              <div className="flex flex-wrap items-end gap-3">
                <div className="crm-mono text-4xl tracking-[-0.05em] text-[var(--crm-accent)]">
                  {shortHash(lastResult.block_hash, 10)}
                </div>
                <ConsolePill tone="good">latest success</ConsolePill>
              </div>
              <div className="mt-4">
                <ConsoleRow label="hash" value={lastResult.block_hash ?? '-'} />
                <ConsoleRow label="nonce" value={lastResult.nonce?.toLocaleString() ?? '-'} />
                <ConsoleRow
                  label="target"
                  value={<TargetValue value={lastResult.target} edge={14} />}
                />
                <ConsoleRow
                  label="next target"
                  value={<TargetValue value={lastResult.next_target} edge={14} />}
                />
                <ConsoleRow
                  label="next difficulty"
                  value={lastResult.next_difficulty ?? '-'}
                />
                <ConsoleRow
                  label="tx included"
                  value={lastResult.transactions.length}
                />
              </div>
            </>
          ) : (
            <ConsoleEmpty
              title="no mined block yet"
              hint="Run a manual mining attempt or enable keep_mining to see accepted block details here."
            />
          )}
        </ConsolePanel>

        <ConsolePanel title="controls" subtitle="runtime toggles" icon="!">
          <div className="space-y-3">
            <ControlCard
              title="Keep mining"
              description="When enabled, the node continues mining as soon as it can assemble a candidate block."
              action={
                <ConsoleButton
                  onClick={handleToggleKeepMining}
                  loading={togglingKeepMining}
                >
                  {keepMining ? 'disable' : 'enable'}
                </ConsoleButton>
              }
            />
            <ControlCard
              title="One-shot mine"
              description="Attempt a single block immediately using the current daemon state."
              action={
                <ConsoleButton
                  tone="primary"
                  onClick={handleMineBlock}
                  loading={mining}
                  loadingText="mining..."
                  disabled={keepMining}
                >
                  mine block now
                </ConsoleButton>
              }
            />
            <ControlCard
              title="Known mining errors"
              description="no transactions in mempool . blockchain empty or inconsistent . block submission rejected"
            />
          </div>
        </ConsolePanel>
      </div>

      <ConsolePanel
        title="transactions included"
        subtitle={`${latestTransactions.length} transactions in latest mined block`}
        icon="tx"
        padded={false}
      >
        {latestTransactions.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="crm-table">
              <thead>
                <tr>
                  <th>tx id</th>
                  <th>type</th>
                  <th>inputs</th>
                  <th>outputs</th>
                  <th className="text-right">value</th>
                </tr>
              </thead>
              <tbody>
                {latestTransactions.map((tx) => (
                  <tr key={tx.id}>
                    <td className="text-[var(--crm-accent)]">
                      {shortHash(tx.id, 12)}
                    </td>
                    <td>
                      <ConsolePill tone={tx.is_coinbase ? 'accent' : 'neutral'}>
                        {tx.is_coinbase ? 'coinbase' : 'transfer'}
                      </ConsolePill>
                    </td>
                    <td>{tx.inputs.length}</td>
                    <td>{tx.outputs.length}</td>
                    <td className="text-right text-[var(--crm-accent)]">
                      {tx.outputs.reduce((total, output) => total + output.value, 0)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <ConsoleEmpty
            title="no mined transaction set"
            hint="Successful mining responses will list the transactions included in the accepted block."
          />
        )}
      </ConsolePanel>
    </div>
  );
}

function Metric({
  label,
  value,
  subtitle,
}: {
  label: string;
  value: ReactNode;
  subtitle: ReactNode;
}) {
  return (
    <div className="min-w-0">
      <div className="crm-field-label">{label}</div>
      <div className="crm-mono min-w-0 text-base text-[var(--crm-fg)]">{value}</div>
      <div className="mt-1 text-xs text-[var(--crm-dim)]">{subtitle}</div>
    </div>
  );
}

function ControlCard({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel-2)] p-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="text-sm font-medium text-[var(--crm-fg)]">{title}</div>
          <div className="mt-1 text-sm text-[var(--crm-dim)]">{description}</div>
        </div>
        {action}
      </div>
    </div>
  );
}
