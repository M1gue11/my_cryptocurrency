import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { useNavigate } from 'react-router-dom';
import { useWallet } from '../contexts';
import {
  ConsoleButton,
  ConsoleEmpty,
  ConsolePageHeader,
  ConsolePanel,
  ConsolePill,
  ConsoleRow,
  ConsoleStat,
  ConsoleStatStrip,
  formatCount,
  formatRelativeTimestamp,
  formatTimestamp,
  shortHash,
  sumTransactionOutputs,
} from '../components';
import { rpcClient } from '../services';
import type {
  BlockInfo,
  ChainStatusResponse,
  MempoolResponse,
  MiningInfoResponse,
  NodeStatusResponse,
  UtxosResponse,
} from '../types';

export function Dashboard() {
  const navigate = useNavigate();
  const { activeWallet } = useWallet();
  const [nodeStatus, setNodeStatus] = useState<NodeStatusResponse | null>(null);
  const [chainStatus, setChainStatus] = useState<ChainStatusResponse | null>(null);
  const [mempool, setMempool] = useState<MempoolResponse | null>(null);
  const [utxos, setUtxos] = useState<UtxosResponse | null>(null);
  const [miningInfo, setMiningInfo] = useState<MiningInfoResponse | null>(null);
  const [blocks, setBlocks] = useState<BlockInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDashboard = useCallback(async (background = false) => {
    try {
      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }
      setError(null);

      const [
        nextNodeStatus,
        nextChainStatus,
        nextMempool,
        nextUtxos,
        nextMiningInfo,
        nextBlocks,
      ] = await Promise.all([
        rpcClient.nodeStatus(),
        rpcClient.chainStatus(),
        rpcClient.nodeMempool(),
        rpcClient.chainUtxos(8),
        rpcClient.mineInfo().catch(() => null),
        rpcClient.chainShow().catch(() => ({ blocks: [] })),
      ]);

      setNodeStatus(nextNodeStatus);
      setChainStatus(nextChainStatus);
      setMempool(nextMempool);
      setUtxos(nextUtxos);
      setMiningInfo(nextMiningInfo);
      setBlocks(nextBlocks.blocks.slice().sort((a, b) => b.height - a.height));
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : 'Failed to connect to the daemon',
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void loadDashboard();
    const interval = setInterval(() => {
      void loadDashboard(true);
    }, 10000);

    return () => clearInterval(interval);
  }, [loadDashboard]);

  const topBlock = blocks[0];
  const activeBalance = activeWallet?.balance?.balance ?? 0;
  const activeUtxoCount = activeWallet?.balance?.utxo_count ?? 0;

  const mempoolTotal = useMemo(
    () =>
      mempool?.transactions.reduce(
        (total, entry) => total + sumTransactionOutputs(entry.tx),
        0,
      ) ?? 0,
    [mempool],
  );

  if (loading && !nodeStatus) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center">
        <div className="crm-mono text-sm text-[var(--crm-dim)]">
          Connecting to daemon...
        </div>
      </div>
    );
  }

  if (error && !nodeStatus) {
    return (
      <div className="flex min-h-[40vh] flex-col items-center justify-center gap-4">
        <div className="max-w-lg text-center text-sm text-[var(--crm-warn)]">
          {error}
        </div>
        <ConsoleButton tone="primary" onClick={() => void loadDashboard()}>
          Retry
        </ConsoleButton>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow="operator overview . node healthy"
        title="Dashboard"
        actions={
          <>
            <ConsolePill>
              local-demo . {new Date().toLocaleDateString()}
            </ConsolePill>
            <ConsoleButton
              onClick={() => void loadDashboard(true)}
              loading={refreshing}
            >
              refresh
            </ConsoleButton>
            <ConsoleButton
              tone="primary"
              onClick={() => navigate('/mining')}
            >
              open mining
            </ConsoleButton>
          </>
        }
      />

      {error ? (
        <div className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
          {error}
        </div>
      ) : null}

      <ConsoleStatStrip columns={6}>
        <ConsoleStat
          label="chain height"
          value={`#${formatCount(nodeStatus?.block_height)}`}
          subtitle={chainStatus?.is_valid ? 'valid' : 'invalid'}
          tone={chainStatus?.is_valid ? 'good' : 'warn'}
        />
        <ConsoleStat
          label="peers"
          value={formatCount(nodeStatus?.peers_connected)}
          subtitle={(nodeStatus?.peers_connected ?? 0) > 0 ? 'connected' : 'no peers'}
          tone={(nodeStatus?.peers_connected ?? 0) > 0 ? 'neutral' : 'warn'}
        />
        <ConsoleStat
          label="mempool"
          value={formatCount(mempool?.count)}
          subtitle={`${mempoolTotal.toFixed(3)} total units`}
          tone={(mempool?.count ?? 0) > 0 ? 'warn' : 'neutral'}
        />
        <ConsoleStat
          label="hashrate"
          value={miningInfo?.is_currently_mining ? 'active' : 'idle'}
          subtitle={miningInfo?.keep_mining_enabled ? 'keep_on' : 'manual'}
          tone={miningInfo?.is_currently_mining ? 'accent' : 'neutral'}
        />
        <ConsoleStat
          label="last block"
          value={chainStatus?.last_block_date ? formatTimestamp(chainStatus.last_block_date) : '-'}
          subtitle={topBlock ? shortHash(topBlock.hash, 6) : 'no chain data'}
        />
        <ConsoleStat
          label="visible utxos"
          value={formatCount(utxos?.utxos.length)}
          subtitle={`${utxos?.total_value ?? 0} units`}
        />
      </ConsoleStatStrip>

      <div className="grid gap-3 xl:grid-cols-[1.1fr_1fr_0.9fr]">
        <ConsolePanel
          title="mining"
          subtitle="mine_info"
          icon="*"
          chip={
            <ConsolePill tone={miningInfo?.keep_mining_enabled ? 'accent' : 'neutral'}>
              keep_mining {miningInfo?.keep_mining_enabled ? 'on' : 'off'}
            </ConsolePill>
          }
        >
          <div className="crm-field-label">
            {miningInfo?.is_currently_mining ? 'searching nonce' : 'idle'}
          </div>
          <div className="crm-mono text-3xl tracking-[-0.04em] text-[var(--crm-accent)]">
            {miningInfo?.started_at ? formatTimestamp(miningInfo.started_at) : '--:--:--'}
          </div>
          <div className="mt-3 h-1 overflow-hidden rounded-full bg-[var(--crm-panel-2)]">
            <div
              className={miningInfo?.is_currently_mining ? 'h-full w-[40%] bg-[var(--crm-accent)]/70' : 'h-full w-0'}
              style={
                miningInfo?.is_currently_mining
                  ? { animation: 'crm-slide 1.8s ease-in-out infinite' }
                  : undefined
              }
            />
          </div>
          <div className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
            <div className="crm-mono text-[var(--crm-dim)]">
              keep_on: {miningInfo?.keep_mining_enabled ? 'true' : 'false'}
            </div>
            <div className="crm-mono text-[var(--crm-dim)]">
              last result: {miningInfo?.last_mined_block?.success ? 'accepted' : 'n/a'}
            </div>
            <div className="crm-mono text-[var(--crm-dim)]">
              started: {miningInfo?.started_at ? formatRelativeTimestamp(miningInfo.started_at) : '-'}
            </div>
            <div className="crm-mono text-[var(--crm-dim)]">
              route: /mining
            </div>
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <ConsoleButton tone="primary" onClick={() => navigate('/mining')}>
              mining center
            </ConsoleButton>
            <ConsoleButton onClick={() => navigate('/blocks')}>
              inspect chain
            </ConsoleButton>
          </div>
        </ConsolePanel>

        <ConsolePanel
          title="top of chain"
          subtitle="chain_status"
          icon="<>"
          action={
            <button
              className="crm-mono text-xs text-[var(--crm-accent)]"
              onClick={() => navigate('/blocks')}
              type="button"
            >
              explore
            </button>
          }
        >
          {topBlock ? (
            <>
              <div className="flex flex-wrap items-end gap-3">
                <div className="crm-mono text-4xl tracking-[-0.05em] text-[var(--crm-accent)]">
                  #{topBlock.height.toLocaleString()}
                </div>
                <ConsolePill tone={chainStatus?.is_valid ? 'good' : 'warn'}>
                  {chainStatus?.is_valid ? 'valid' : 'invalid'}
                </ConsolePill>
              </div>
              <div className="mt-4">
                <ConsoleRow label="hash" value={topBlock.hash} />
                <ConsoleRow label="prev" value={topBlock.prev_hash} />
                <ConsoleRow label="merkle" value={topBlock.merkle_root} />
                <ConsoleRow label="nonce" value={topBlock.nonce.toLocaleString()} />
                <ConsoleRow
                  label="txs"
                  value={`${topBlock.transactions.length} (${topBlock.transactions.filter((tx) => tx.is_coinbase).length} coinbase)`}
                />
              </div>
            </>
          ) : (
            <ConsoleEmpty
              title="no block data"
              hint="The explorer view will populate once the daemon returns chain data."
            />
          )}
        </ConsolePanel>

        <ConsolePanel
          title="active wallet"
          subtitle="session"
          icon="[]"
          action={
            <button
              className="crm-mono text-xs text-[var(--crm-accent)]"
              onClick={() => navigate('/wallet')}
              type="button"
            >
              open
            </button>
          }
        >
          {activeWallet ? (
            <>
              <div className="crm-field-label">
                {activeWallet.keyPath} . loaded
              </div>
              <div className="crm-mono text-3xl tracking-[-0.04em]">
                {activeBalance.toFixed(3)}{' '}
                <span className="text-sm text-[var(--crm-dim)]">units</span>
              </div>
              <div className="mt-2 text-sm text-[var(--crm-muted)]">
                {shortHash(activeWallet.address, 10)} . {activeUtxoCount} UTXOs
              </div>
              <div className="mt-4 flex flex-wrap gap-2">
                <ConsoleButton
                  tone="primary"
                  onClick={() => navigate('/wallet?tab=send')}
                >
                  send
                </ConsoleButton>
                <ConsoleButton onClick={() => navigate('/wallet?tab=receive')}>
                  receive
                </ConsoleButton>
              </div>
              <div className="mt-4 rounded-sm border border-dashed border-[var(--crm-border)] bg-[var(--crm-panel-2)] px-3 py-3 text-sm text-[var(--crm-dim)]">
                Session-backed wallet access lives entirely in the frontend; no
                password is written to disk.
              </div>
            </>
          ) : (
            <ConsoleEmpty
              title="no wallet loaded"
              hint="Import or create a wallet to unlock send, receive, and key management."
              action={
                <ConsoleButton tone="primary" onClick={() => navigate('/wallet')}>
                  open wallet
                </ConsoleButton>
              }
            />
          )}
        </ConsolePanel>
      </div>

      <div className="grid gap-3 xl:grid-cols-[1.4fr_1fr]">
        <ConsolePanel
          title="recent blocks"
          subtitle={`chain_show . last ${Math.min(blocks.length, 8)}`}
          icon="#"
          padded={false}
          action={
            <button
              className="crm-mono text-xs text-[var(--crm-accent)]"
              onClick={() => navigate('/blocks')}
              type="button"
            >
              all
            </button>
          }
        >
          {blocks.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="crm-table crm-table--interactive">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>hash</th>
                    <th>txs</th>
                    <th>size</th>
                    <th>time</th>
                  </tr>
                </thead>
                <tbody>
                  {blocks.slice(0, 8).map((block) => (
                    <tr
                      key={block.hash}
                      onClick={() => navigate(`/blocks?height=${block.height}`)}
                    >
                      <td className="text-[var(--crm-accent)]">
                        {block.height}
                      </td>
                      <td>{shortHash(block.hash, 10)}</td>
                      <td>{block.transactions.length}</td>
                      <td>{(block.size_bytes / 1024).toFixed(1)} kB</td>
                      <td className="text-[var(--crm-dim)]">
                        {formatRelativeTimestamp(block.timestamp)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <ConsoleEmpty title="no blocks returned" />
          )}
        </ConsolePanel>

        <ConsolePanel
          title="mempool"
          subtitle={`${mempool?.count ?? 0} pending`}
          icon="tx"
          padded={false}
          action={
            <button
              className="crm-mono text-xs text-[var(--crm-accent)]"
              onClick={() => navigate('/transactions')}
              type="button"
            >
              inspect
            </button>
          }
        >
          {mempool && mempool.transactions.length > 0 ? (
            <div>
              {mempool.transactions.slice(0, 5).map((entry) => (
                <div
                  key={entry.tx.id}
                  className="border-t border-[var(--crm-border)] px-4 py-3 first:border-t-0"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="crm-mono text-sm text-[var(--crm-muted)]">
                      {shortHash(entry.tx.id, 12)}
                    </div>
                    <div className="crm-mono text-sm text-[var(--crm-accent)]">
                      {sumTransactionOutputs(entry.tx).toFixed(3)} units
                    </div>
                  </div>
                  <div className="mt-1 flex flex-wrap justify-between gap-2 text-xs text-[var(--crm-dim)]">
                    <span>
                      {entry.tx.inputs.length} inputs . {entry.tx.outputs.length}{' '}
                      outputs
                    </span>
                    <span>{entry.tx.size} bytes</span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <ConsoleEmpty
              title="mempool empty"
              hint="Submitted transactions will queue here before being mined."
            />
          )}
        </ConsolePanel>
      </div>
    </div>
  );
}
