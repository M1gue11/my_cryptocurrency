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
  shortHash,
  sumTransactionOutputs,
} from '../components';
import { rpcClient } from '../services';
import type { MempoolResponse, TransactionViewResponse } from '../types';

export function Transactions() {
  const [mempool, setMempool] = useState<MempoolResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchId, setSearchId] = useState('');
  const [searchResult, setSearchResult] = useState<TransactionViewResponse | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);

  const loadMempool = useCallback(async (background = false) => {
    try {
      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }
      setError(null);
      const response = await rpcClient.nodeMempool();
      setMempool(response);
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : 'Failed to fetch mempool data',
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void loadMempool();
    const interval = setInterval(() => {
      void loadMempool(true);
    }, 10000);
    return () => clearInterval(interval);
  }, [loadMempool]);

  const handleSearch = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!searchId.trim()) return;

    try {
      setSearching(true);
      setSearchError(null);
      setSearchResult(null);
      const result = await rpcClient.transactionView(searchId.trim());
      setSearchResult(result);
    } catch (nextError) {
      setSearchError(
        nextError instanceof Error ? nextError.message : 'Transaction not found',
      );
    } finally {
      setSearching(false);
    }
  };

  const clearSearch = () => {
    setSearchId('');
    setSearchResult(null);
    setSearchError(null);
  };

  const clearMempool = async () => {
    try {
      setRefreshing(true);
      await rpcClient.nodeClearMempool();
      await loadMempool(true);
    } catch (nextError) {
      setError(
        nextError instanceof Error ? nextError.message : 'Failed to clear mempool',
      );
      setRefreshing(false);
    }
  };

  const mempoolTotal = useMemo(
    () =>
      mempool?.transactions.reduce(
        (total, entry) => total + sumTransactionOutputs(entry.tx),
        0,
      ) ?? 0,
    [mempool],
  );

  if (loading && !mempool) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center">
        <div className="crm-mono text-sm text-[var(--crm-dim)]">
          Loading transaction workspace...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow="transaction_view . node_mempool"
        title="Transactions"
        actions={
          <>
            <ConsoleButton onClick={() => void loadMempool(true)} loading={refreshing}>
              refresh
            </ConsoleButton>
            <ConsoleButton
              tone="danger"
              onClick={() => void clearMempool()}
              disabled={(mempool?.count ?? 0) === 0}
            >
              clear mempool
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
          label="pending tx"
          value={mempool?.count ?? 0}
          subtitle="currently in mempool"
          tone={(mempool?.count ?? 0) > 0 ? 'warn' : 'neutral'}
        />
        <ConsoleStat
          label="pending value"
          value={mempoolTotal.toFixed(3)}
          subtitle="output sum"
          tone="accent"
        />
        <ConsoleStat
          label="searched tx"
          value={searchResult ? shortHash(searchResult.id, 10) : '-'}
          subtitle={searchResult ? `${searchResult.size} bytes` : 'none selected'}
        />
        <ConsoleStat
          label="type"
          value={
            searchResult
              ? searchResult.is_coinbase
                ? 'coinbase'
                : 'transfer'
              : '-'
          }
          subtitle={searchResult?.message || 'transaction metadata'}
        />
      </ConsoleStatStrip>

      <div className="grid gap-3 xl:grid-cols-[1.15fr_1fr]">
        <ConsolePanel title="lookup transaction" subtitle="transaction_view" icon="?">
          <form className="space-y-4" onSubmit={handleSearch}>
            <div>
              <div className="crm-field-label">transaction id</div>
              <input
                className="crm-input"
                value={searchId}
                onChange={(event) => setSearchId(event.target.value)}
                placeholder="Paste a full transaction id"
              />
            </div>
            <div className="flex flex-wrap gap-2">
              <ConsoleButton tone="primary" type="submit" loading={searching}>
                search
              </ConsoleButton>
              {(searchResult || searchError) && (
                <ConsoleButton onClick={clearSearch} type="button">
                  clear
                </ConsoleButton>
              )}
            </div>
          </form>

          {searchError ? (
            <div className="mt-4 rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
              {searchError}
            </div>
          ) : null}

          {searchResult ? (
            <div className="mt-5 space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <ConsolePill tone={searchResult.is_coinbase ? 'accent' : 'neutral'}>
                  {searchResult.is_coinbase ? 'coinbase' : 'transfer'}
                </ConsolePill>
                <ConsolePill>{searchResult.size} bytes</ConsolePill>
              </div>
              <ConsoleRow label="id" value={searchResult.id} />
              <ConsoleRow label="date" value={searchResult.date} />
              <ConsoleRow
                label="message"
                value={searchResult.message || '-'}
                mono={false}
              />
              <ConsoleRow
                label="total out"
                value={`${sumTransactionOutputs(searchResult).toFixed(3)} units`}
              />
            </div>
          ) : (
            <div className="mt-5">
              <ConsoleEmpty
                title="no transaction loaded"
                hint="Search by full id to inspect a confirmed or pending transaction."
              />
            </div>
          )}
        </ConsolePanel>

        <ConsolePanel title="transaction structure" subtitle="inputs + outputs" icon="[]">
          {searchResult ? (
            <div className="space-y-4">
              <div>
                <div className="crm-field-label">
                  inputs ({searchResult.inputs.length})
                </div>
                {searchResult.inputs.length > 0 ? (
                  <div className="space-y-2">
                    {searchResult.inputs.map((input) => (
                      <div
                        key={`${input.prev_tx_id}-${input.output_index}`}
                        className="rounded-sm border border-dashed border-[var(--crm-border)] px-4 py-3"
                      >
                        <div className="crm-mono text-sm">
                          {shortHash(input.prev_tx_id, 14)}
                        </div>
                        <div className="mt-1 text-xs text-[var(--crm-dim)]">
                          output index {input.output_index}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <ConsoleEmpty
                    title="no inputs"
                    hint="This transaction is a coinbase reward."
                  />
                )}
              </div>

              <div>
                <div className="crm-field-label">
                  outputs ({searchResult.outputs.length})
                </div>
                <div className="space-y-2">
                  {searchResult.outputs.map((output, index) => (
                    <div
                      key={`${output.address}-${index}`}
                      className="rounded-sm border border-dashed border-[var(--crm-border)] px-4 py-3"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="crm-mono text-sm">
                          {shortHash(output.address, 16)}
                        </div>
                        <div className="crm-mono text-sm text-[var(--crm-accent)]">
                          {output.value}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          ) : (
            <ConsoleEmpty
              title="transaction detail unavailable"
              hint="Search above or click an item in the mempool table to inspect its inputs and outputs."
            />
          )}
        </ConsolePanel>
      </div>

      <ConsolePanel
        title="mempool"
        subtitle={`${mempool?.count ?? 0} pending`}
        icon="tx"
        padded={false}
      >
        {mempool && mempool.transactions.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="crm-table crm-table--interactive">
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
                  <tr
                    key={entry.tx.id}
                    onClick={() => {
                      setSearchId(entry.tx.id);
                      setSearchResult(entry.tx);
                      setSearchError(null);
                    }}
                  >
                    <td className="text-[var(--crm-accent)]">
                      {shortHash(entry.tx.id, 14)}
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
          <ConsoleEmpty
            title="mempool empty"
            hint="Transactions submitted from wallets will appear here before they are mined."
          />
        )}
      </ConsolePanel>
    </div>
  );
}
