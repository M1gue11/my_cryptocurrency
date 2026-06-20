import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'motion/react';
import {
  AnimatedNumber,
  ConsoleButton,
  ConsoleEmpty,
  ConsolePageHeader,
  ConsolePanel,
  ConsolePill,
  ConsoleRow,
  ConsoleStat,
  ConsoleStatStrip,
  ElapsedTimer,
  HashDisplay,
  MetricCard,
  formatValue,
  parseApiDate,
} from '../components';
import { MiningRing } from '../components/display/MiningRing';
import { rpcClient } from '../services';
import type { MineBlockResponse, MiningInfoResponse } from '../types';

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
  const shouldReduce = useReducedMotion();

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
    <div className="crm-page space-y-5">
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

      <AnimatePresence>
        {error ? (
          <motion.div
            key="error"
            className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]"
            initial={{ opacity: 0, y: -8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ type: 'spring', stiffness: 380, damping: 28 }}
          >
            {error}
          </motion.div>
        ) : null}
      </AnimatePresence>

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
          label="elapsed"
          value={
            <ElapsedTimer
              seconds={elapsedSeconds}
              active={Boolean(miningInfo.started_at)}
              size="md"
            />
          }
          subtitle={
            miningInfo.started_at
              ? `started ${parseApiDate(miningInfo.started_at).toLocaleTimeString()}`
              : 'no active session'
          }
        />
        <ConsoleStat
          label="last target"
          value={<HashDisplay value={lastResult?.target} preset="stat" size="sm" />}
          subtitle={
            lastResult?.next_target ? (
              <HashDisplay
                value={lastResult.next_target}
                preset="table"
                size="xs"
                className="text-[var(--crm-dim)]"
              />
            ) : (
              'awaiting mined block'
            )
          }
        />
      </ConsoleStatStrip>

      <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-5">
        <div className="grid gap-6 lg:grid-cols-[auto_1fr]">
          <div className="flex items-center gap-5">
            <MiningRing active={isCurrentlyMining} size={96} />

            <div>
              <div className="crm-field-label">{derivedStatusLabel}</div>
              <ElapsedTimer
                seconds={elapsedSeconds}
                active={Boolean(miningInfo.started_at)}
                size="xl"
              />
              <div className="mt-1 text-sm text-[var(--crm-dim)]">
                {miningInfo.started_at
                  ? `started at ${parseApiDate(miningInfo.started_at).toLocaleTimeString()}`
                  : 'No active mining session'}
              </div>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricCard
              label="target"
              value={<HashDisplay value={lastResult?.target} preset="detail" size="md" />}
              subtitle="current pow target"
            />
            <MetricCard
              label="difficulty"
              value={lastResult?.next_difficulty ?? '-'}
              subtitle="next block difficulty"
            />
            <MetricCard
              label="last block"
              value={
                <HashDisplay
                  value={lastResult?.block_hash}
                  preset="detail"
                  size="md"
                />
              }
              subtitle="accepted block hash"
            />
            <MetricCard
              label="nonce"
              value={<AnimatedNumber value={lastResult?.nonce ?? null} />}
              subtitle="most recent result"
            />
          </div>
        </div>
      </div>

      <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-4">
        <div className="mb-3 flex flex-wrap items-center gap-3">
          <div className="crm-field-label">mining progress</div>
          <div className="ml-auto crm-mono text-xs text-[var(--crm-dim)]">
            {isCurrentlyMining
              ? 'trying nonces… waiting for valid target'
              : 'miner idle — toggle keep_mining or run a manual attempt'}
          </div>
        </div>
        <div className="relative h-2 overflow-hidden rounded-full bg-[var(--crm-panel-2)]">
          {isCurrentlyMining ? (
            shouldReduce ? (
              <div className="absolute inset-y-0 w-[40%] bg-[var(--crm-accent)] opacity-60" />
            ) : (
              <motion.div
                className="absolute inset-y-0 w-[40%] bg-[linear-gradient(90deg,transparent,var(--crm-accent),transparent)]"
                animate={{ x: ['-100%', '250%'] }}
                transition={{ duration: 1.8, repeat: Infinity, ease: 'easeInOut' }}
              />
            )
          ) : null}
        </div>
        <div className="mt-3 flex flex-wrap justify-between gap-2 crm-mono text-[10px] text-[var(--crm-dim)]">
          <ProgressStep active={isCurrentlyMining} done>
            candidate block
          </ProgressStep>
          <ProgressStep active={isCurrentlyMining} done>
            transactions
          </ProgressStep>
          <ProgressStep active={isCurrentlyMining} highlight>
            hashing header
          </ProgressStep>
          <ProgressStep active={isCurrentlyMining}>
            valid nonce
          </ProgressStep>
          <ProgressStep active={isCurrentlyMining}>
            submitted
          </ProgressStep>
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
                <HashDisplay
                  value={lastResult.block_hash}
                  preset="detail"
                  size="lg"
                  className="text-[var(--crm-accent)]"
                />
                <ConsolePill tone="good">latest success</ConsolePill>
              </div>
              <div className="mt-4">
                <ConsoleRow label="hash" value={lastResult.block_hash ?? '-'} hash />
                <ConsoleRow
                  label="nonce"
                  value={<AnimatedNumber value={lastResult.nonce ?? null} />}
                />
                <ConsoleRow
                  label="target"
                  value={lastResult.target ?? '-'}
                  hash
                />
                <ConsoleRow
                  label="next target"
                  value={lastResult.next_target ?? '-'}
                  hash
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
              description="no transactions in mempool · blockchain empty or inconsistent · block submission rejected"
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
                    <td>
                      <HashDisplay value={tx.id} preset="table" size="sm" />
                    </td>
                    <td>
                      <ConsolePill tone={tx.is_coinbase ? 'accent' : 'neutral'}>
                        {tx.is_coinbase ? 'coinbase' : 'transfer'}
                      </ConsolePill>
                    </td>
                    <td>{tx.inputs.length}</td>
                    <td>{tx.outputs.length}</td>
                    <td className="text-right text-[var(--crm-accent)]">
                      {formatValue(
                        tx.outputs.reduce((total, output) => total + output.value, 0),
                        { suffix: '' },
                      )}
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

function ProgressStep({
  children,
  active,
  done,
  highlight,
}: {
  children: ReactNode;
  active: boolean;
  done?: boolean;
  highlight?: boolean;
}) {
  const lit = active && (done || highlight);
  return (
    <span className={lit ? 'text-[var(--crm-accent)]' : ''}>
      {children}
    </span>
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
