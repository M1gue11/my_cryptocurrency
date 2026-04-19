import {
  useCallback,
  useEffect,
  useMemo,
  useState,
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
  formatTimestamp,
  shortHash,
  sumTransactionOutputs,
} from '../components';
import { rpcClient } from '../services';
import type { MempoolResponse, NodeStatusResponse, PeerInfo } from '../types';

type ConfirmAction = 'save' | 'clearMempool' | 'restart' | null;

export function Network() {
  const [nodeStatus, setNodeStatus] = useState<NodeStatusResponse | null>(null);
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [mempool, setMempool] = useState<MempoolResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null);
  const [disconnectingPeer, setDisconnectingPeer] = useState<string | null>(null);

  const loadRuntime = useCallback(async (background = false) => {
    try {
      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }
      setError(null);

      const [statusResponse, peerResponse, mempoolResponse] = await Promise.all([
        rpcClient.nodeStatus(),
        rpcClient.peersList(),
        rpcClient.nodeMempool(),
      ]);

      setNodeStatus(statusResponse);
      setPeers(peerResponse.peers);
      setMempool(mempoolResponse);
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : 'Failed to fetch node runtime information',
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void loadRuntime();
    const interval = setInterval(() => {
      void loadRuntime(true);
    }, 8000);
    return () => clearInterval(interval);
  }, [loadRuntime]);

  const mempoolValue = useMemo(
    () =>
      mempool?.transactions.reduce(
        (total, entry) => total + sumTransactionOutputs(entry.tx),
        0,
      ) ?? 0,
    [mempool],
  );

  const handleDisconnectPeer = async (addr: string) => {
    try {
      setDisconnectingPeer(addr);
      setActionMessage(null);
      const response = await rpcClient.peerDisconnect(addr);
      setActionMessage(response.message ?? `Disconnect requested for ${addr}`);
      await loadRuntime(true);
    } catch (nextError) {
      setActionMessage(
        nextError instanceof Error ? nextError.message : `Failed to disconnect ${addr}`,
      );
    } finally {
      setDisconnectingPeer(null);
    }
  };

  const runConfirmedAction = async () => {
    if (!confirmAction) return;

    try {
      if (confirmAction === 'save') {
        await rpcClient.nodeSave();
        setActionMessage('node_save -> state persisted');
      }

      if (confirmAction === 'clearMempool') {
        await rpcClient.nodeClearMempool();
        setActionMessage('node_clear_mempool -> pending transactions dropped');
      }

      if (confirmAction === 'restart') {
        await rpcClient.nodeInit();
        setActionMessage('node_init -> runtime reinitialized');
      }

      setConfirmAction(null);
      await loadRuntime(true);
    } catch (nextError) {
      setActionMessage(
        nextError instanceof Error ? nextError.message : 'Action failed',
      );
      setConfirmAction(null);
    }
  };

  if (loading && !nodeStatus) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center">
        <div className="crm-mono text-sm text-[var(--crm-dim)]">
          Loading runtime information...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow="node_status . peers_list . node_save"
        title="Node / Runtime"
        actions={
          <ConsoleButton onClick={() => void loadRuntime(true)} loading={refreshing}>
            refresh
          </ConsoleButton>
        }
      />

      {error ? (
        <div className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
          {error}
        </div>
      ) : null}

      {actionMessage ? (
        <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] px-4 py-3 text-sm text-[var(--crm-muted)]">
          {actionMessage}
        </div>
      ) : null}

      <ConsoleStatStrip columns={4}>
        <ConsoleStat
          label="version"
          value={nodeStatus?.version ?? '-'}
          subtitle="daemon"
        />
        <ConsoleStat
          label="peers"
          value={nodeStatus?.peers_connected ?? 0}
          subtitle="connected"
          tone={(nodeStatus?.peers_connected ?? 0) > 0 ? 'neutral' : 'warn'}
        />
        <ConsoleStat
          label="chain tip"
          value={`#${nodeStatus?.block_height.toLocaleString() ?? '-'}`}
          subtitle={shortHash(nodeStatus?.top_block_hash, 8)}
          tone="accent"
        />
        <ConsoleStat
          label="mempool"
          value={mempool?.count ?? 0}
          subtitle={`${mempoolValue.toFixed(3)} units`}
          tone={(mempool?.count ?? 0) > 0 ? 'warn' : 'neutral'}
        />
      </ConsoleStatStrip>

      <div className="grid gap-3 xl:grid-cols-[1.2fr_1fr]">
        <ConsolePanel title="status" subtitle="node_status" icon="[]">
          <ConsoleRow label="daemon" value="running" />
          <ConsoleRow label="version" value={nodeStatus?.version ?? '-'} />
          <ConsoleRow label="rpc endpoint" value="http://localhost:7001/rpc" />
          <ConsoleRow label="p2p peers" value={nodeStatus?.peers_connected ?? 0} />
          <ConsoleRow label="tip hash" value={nodeStatus?.top_block_hash ?? '-'} />
          <ConsoleRow
            label="tip height"
            value={nodeStatus ? `#${nodeStatus.block_height}` : '-'}
          />
        </ConsolePanel>

        <ConsolePanel
          title="administrative actions"
          subtitle="confirm before running"
          icon="!"
        >
          <ActionCard
            title="Save node state"
            description="Persist the current blockchain, mempool, and UTXO state to disk."
            buttonLabel="node_save"
            tone="primary"
            onClick={() => setConfirmAction('save')}
          />
          <ActionCard
            title="Clear mempool"
            description="Drop all pending transactions. Wallets must re-broadcast if needed."
            buttonLabel="node_clear_mempool"
            tone="danger"
            onClick={() => setConfirmAction('clearMempool')}
          />
          <ActionCard
            title="Reinitialize node"
            description="Restart runtime state from disk and reset the live daemon process."
            buttonLabel="node_init"
            tone="danger"
            onClick={() => setConfirmAction('restart')}
          />
        </ConsolePanel>
      </div>

      <ConsolePanel
        title="peer connections"
        subtitle={`${peers.length} active peers`}
        icon="@"
        padded={false}
      >
        {peers.length > 0 ? (
          <div className="space-y-0">
            {peers.map((peer) => (
              <div
                key={peer.addr}
                className="border-t border-[var(--crm-border)] px-4 py-4 first:border-t-0"
              >
                <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                  <div className="space-y-3">
                    <div>
                      <div className="crm-field-label">peer</div>
                      <div className="crm-mono text-sm">{peer.addr}</div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <ConsolePill tone={peer.direction === 'outbound' ? 'accent' : 'neutral'}>
                        {peer.direction}
                      </ConsolePill>
                      <ConsolePill
                        tone={
                          peer.connection_state === 'disconnecting' ? 'warn' : 'good'
                        }
                      >
                        {peer.connection_state}
                      </ConsolePill>
                      <ConsolePill
                        tone={
                          peer.handshake_state === 'handshake_complete'
                            ? 'good'
                            : peer.handshake_state === 'version_received'
                              ? 'warn'
                              : 'neutral'
                        }
                      >
                        {peer.handshake_state}
                      </ConsolePill>
                    </div>
                    <div className="grid gap-3 text-sm text-[var(--crm-dim)] md:grid-cols-3">
                      <div>connected: {formatTimestamp(peer.connected_at)}</div>
                      <div>last event: {formatTimestamp(peer.last_event_at)}</div>
                      <div>{peer.last_event ?? 'No events recorded'}</div>
                    </div>
                  </div>

                  <ConsoleButton
                    tone="danger"
                    size="sm"
                    loading={disconnectingPeer === peer.addr}
                    loadingText="Disconnecting..."
                    onClick={() => void handleDisconnectPeer(peer.addr)}
                    disabled={peer.connection_state === 'disconnecting'}
                  >
                    disconnect
                  </ConsoleButton>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <ConsoleEmpty
            title="no active peers"
            hint="Inbound and outbound P2P connections will appear here once the node joins a network."
          />
        )}
      </ConsolePanel>

      <ConsolePanel
        title="mempool contents"
        subtitle={`${mempool?.count ?? 0} transactions`}
        icon="tx"
        padded={false}
        action={
          (mempool?.count ?? 0) > 0 ? (
            <ConsoleButton
              tone="danger"
              size="sm"
              onClick={() => setConfirmAction('clearMempool')}
            >
              clear all
            </ConsoleButton>
          ) : null
        }
      >
        {mempool && mempool.transactions.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="crm-table">
              <thead>
                <tr>
                  <th>tx id</th>
                  <th>inputs</th>
                  <th>outputs</th>
                  <th>message</th>
                  <th className="text-right">value</th>
                </tr>
              </thead>
              <tbody>
                {mempool.transactions.map((entry) => (
                  <tr key={entry.tx.id}>
                    <td className="text-[var(--crm-accent)]">
                      {shortHash(entry.tx.id, 12)}
                    </td>
                    <td>{entry.tx.inputs.length}</td>
                    <td>{entry.tx.outputs.length}</td>
                    <td className="text-[var(--crm-muted)]">
                      {entry.tx.message || '-'}
                    </td>
                    <td className="text-right text-[var(--crm-accent)]">
                      {sumTransactionOutputs(entry.tx).toFixed(3)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <ConsoleEmpty title="mempool empty" />
        )}
      </ConsolePanel>

      {confirmAction ? (
        <div
          className="fixed inset-0 z-50 grid place-items-center bg-black/60 px-4"
          onClick={() => setConfirmAction(null)}
        >
          <div
            className="w-full max-w-lg rounded-sm border border-[var(--crm-border-strong)] bg-[var(--crm-panel)] p-5 shadow-[var(--crm-shadow)]"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="crm-eyebrow text-[var(--crm-warn)]">confirm action</div>
            <div className="mt-2 text-lg font-medium text-[var(--crm-fg)]">
              {confirmAction === 'save' && 'Persist state to disk?'}
              {confirmAction === 'clearMempool' && 'Clear the mempool?'}
              {confirmAction === 'restart' && 'Reinitialize the node?'}
            </div>
            <div className="mt-3 text-sm leading-6 text-[var(--crm-muted)]">
              {confirmAction === 'save' &&
                'This writes the current blockchain, mempool, and UTXO set to disk. It is safe to run at any time.'}
              {confirmAction === 'clearMempool' &&
                'All pending transactions will be dropped and must be re-broadcast by wallets if they should remain pending.'}
              {confirmAction === 'restart' &&
                'The daemon will rebuild live runtime state from disk and peers may disconnect during the process.'}
            </div>
            <div className="mt-5 flex justify-end gap-2">
              <ConsoleButton onClick={() => setConfirmAction(null)}>
                cancel
              </ConsoleButton>
              <ConsoleButton
                tone={confirmAction === 'save' ? 'primary' : 'danger'}
                onClick={() => void runConfirmedAction()}
              >
                confirm
              </ConsoleButton>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

interface ActionCardProps {
  title: string;
  description: string;
  buttonLabel: string;
  tone: 'primary' | 'danger';
  onClick: () => void;
}

function ActionCard({
  title,
  description,
  buttonLabel,
  tone,
  onClick,
}: ActionCardProps) {
  return (
    <div className="mb-3 rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel-2)] p-4 last:mb-0">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="text-sm font-medium text-[var(--crm-fg)]">{title}</div>
          <div className="mt-1 text-sm text-[var(--crm-dim)]">{description}</div>
        </div>
        <ConsoleButton tone={tone} size="sm" onClick={onClick}>
          {buttonLabel}
        </ConsoleButton>
      </div>
    </div>
  );
}
