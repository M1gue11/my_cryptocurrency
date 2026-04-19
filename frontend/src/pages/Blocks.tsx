import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { useSearchParams } from 'react-router-dom';
import {
  ConsoleButton,
  ConsoleEmpty,
  ConsolePageHeader,
  ConsolePanel,
  ConsolePill,
  ConsoleRow,
  ConsoleStat,
  ConsoleStatStrip,
  ConsoleTabs,
  formatRelativeTimestamp,
  shortHash,
  sumTransactionOutputs,
} from '../components';
import { rpcClient } from '../services';
import type { BlockInfo, TransactionViewResponse, UtxosResponse } from '../types';

type ExplorerTab = 'blocks' | 'utxos';
type SearchMode = 'hash' | 'height';

export function Blocks() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [blocks, setBlocks] = useState<BlockInfo[]>([]);
  const [utxos, setUtxos] = useState<UtxosResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [validationMessage, setValidationMessage] = useState<string | null>(null);
  const [selectedBlock, setSelectedBlock] = useState<BlockInfo | null>(null);
  const [selectedTx, setSelectedTx] = useState<TransactionViewResponse | null>(null);
  const [tab, setTab] = useState<ExplorerTab>('blocks');
  const [searchMode, setSearchMode] = useState<SearchMode>('hash');
  const [searchQuery, setSearchQuery] = useState('');

  const loadExplorer = useCallback(async (background = false) => {
    try {
      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }
      setError(null);

      const [blockResponse, utxoResponse] = await Promise.all([
        rpcClient.chainShow(),
        rpcClient.chainUtxos(50),
      ]);

      const nextBlocks = blockResponse.blocks
        .slice()
        .sort((a, b) => b.height - a.height);

      setBlocks(nextBlocks);
      setUtxos(utxoResponse);
    } catch (nextError) {
      setError(
        nextError instanceof Error ? nextError.message : 'Failed to fetch chain data',
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void loadExplorer();
  }, [loadExplorer]);

  useEffect(() => {
    const heightParam = searchParams.get('height');
    if (!heightParam || blocks.length === 0) return;

    const parsedHeight = Number(heightParam);
    const matchingBlock = blocks.find((block) => block.height === parsedHeight);
    if (matchingBlock) {
      setSelectedBlock(matchingBlock);
      setSelectedTx(null);
    }
  }, [blocks, searchParams]);

  const filteredBlocks = useMemo(() => {
    if (!searchQuery.trim()) return blocks;
    const query = searchQuery.trim().toLowerCase();

    if (searchMode === 'height') {
      const targetHeight = Number(query);
      return Number.isFinite(targetHeight)
        ? blocks.filter((block) => block.height === targetHeight)
        : blocks;
    }

    return blocks.filter((block) => block.hash.toLowerCase().includes(query));
  }, [blocks, searchMode, searchQuery]);

  const selectBlock = (block: BlockInfo) => {
    setSelectedBlock(block);
    setSelectedTx(null);
    setSearchParams({ height: String(block.height) });
  };

  const clearSelection = () => {
    setSelectedBlock(null);
    setSelectedTx(null);
    setSearchParams({});
  };

  const validateChain = async () => {
    try {
      const response = await rpcClient.chainValidate();
      setValidationMessage(
        response.valid ? 'chain_validate -> valid' : response.error ?? 'chain invalid',
      );
    } catch (nextError) {
      setValidationMessage(
        nextError instanceof Error ? nextError.message : 'Validation failed',
      );
    }
  };

  if (loading && blocks.length === 0) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center">
        <div className="crm-mono text-sm text-[var(--crm-dim)]">
          Loading blockchain data...
        </div>
      </div>
    );
  }

  const currentBlock = selectedBlock;
  const selectedTxTotal = selectedTx ? sumTransactionOutputs(selectedTx) : 0;

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow={
          currentBlock
            ? selectedTx
              ? 'transaction_view'
              : 'chain_show'
            : 'chain_show . chain_utxos'
        }
        title={
          currentBlock
            ? selectedTx
              ? 'Transaction'
              : `Block #${currentBlock.height}`
            : 'Blockchain'
        }
        actions={
          <>
            {(currentBlock || selectedTx) && (
              <ConsoleButton onClick={clearSelection}>back</ConsoleButton>
            )}
            <ConsoleButton onClick={() => void loadExplorer(true)} loading={refreshing}>
              refresh
            </ConsoleButton>
            <ConsoleButton tone="primary" onClick={validateChain}>
              chain_validate
            </ConsoleButton>
          </>
        }
      />

      {error ? (
        <div className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
          {error}
        </div>
      ) : null}

      {validationMessage ? (
        <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] px-4 py-3 text-sm text-[var(--crm-muted)]">
          {validationMessage}
        </div>
      ) : null}

      {!currentBlock && (
        <>
          <ConsoleStatStrip columns={5}>
            <ConsoleStat
              label="height"
              value={`#${blocks[0]?.height.toLocaleString() ?? '-'}`}
              subtitle="current tip"
              tone="accent"
            />
            <ConsoleStat
              label="latest hash"
              value={shortHash(blocks[0]?.hash, 10)}
              subtitle="chain_status"
            />
            <ConsoleStat
              label="validity"
              value="valid"
              subtitle="integrity check"
              tone="good"
            />
            <ConsoleStat
              label="head tx count"
              value={blocks[0]?.transactions.length ?? 0}
              subtitle={`${blocks[0]?.size_bytes ?? 0} bytes`}
            />
            <ConsoleStat
              label="utxos"
              value={utxos?.utxos.length ?? 0}
              subtitle={`${utxos?.total_value ?? 0} units`}
            />
          </ConsoleStatStrip>

          <ConsoleTabs
            active={tab}
            onChange={setTab}
            items={[
              { key: 'blocks', label: 'Blocks' },
              { key: 'utxos', label: 'UTXOs' },
            ]}
            trailing={
              tab === 'blocks'
                ? `showing ${Math.min(filteredBlocks.length, 20)} of ${blocks.length}`
                : `${utxos?.utxos.length ?? 0} outputs`
            }
          />

          {tab === 'blocks' ? (
            <ConsolePanel
              title="blocks"
              subtitle="freshest first"
              icon="#"
              padded={false}
            >
              <div className="border-b border-[var(--crm-border)] p-4">
                <div className="flex flex-col gap-3 md:flex-row">
                  <input
                    className="crm-input md:flex-1"
                    type={searchMode === 'height' ? 'number' : 'text'}
                    placeholder={
                      searchMode === 'height'
                        ? 'Search by block height'
                        : 'Search by block hash'
                    }
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                  />
                  <ConsoleButton
                    onClick={() => {
                      setSearchMode(searchMode === 'hash' ? 'height' : 'hash');
                      setSearchQuery('');
                    }}
                  >
                    {searchMode === 'hash' ? '# height' : 'hash'}
                  </ConsoleButton>
                </div>
              </div>
              <div className="overflow-x-auto">
                <table className="crm-table crm-table--interactive">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>hash</th>
                      <th>prev</th>
                      <th>txs</th>
                      <th>size</th>
                      <th>nonce</th>
                      <th>time</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredBlocks.slice(0, 20).map((block) => (
                      <tr key={block.hash} onClick={() => selectBlock(block)}>
                        <td className="text-[var(--crm-accent)]">{block.height}</td>
                        <td>{shortHash(block.hash, 12)}</td>
                        <td className="text-[var(--crm-muted)]">
                          {shortHash(block.prev_hash, 12)}
                        </td>
                        <td>{block.transactions.length}</td>
                        <td>{(block.size_bytes / 1024).toFixed(2)} kB</td>
                        <td>{block.nonce.toLocaleString()}</td>
                        <td className="text-[var(--crm-dim)]">
                          {formatRelativeTimestamp(block.timestamp)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {filteredBlocks.length === 0 ? (
                <ConsoleEmpty
                  title="no matching blocks"
                  hint="Try a different height or a shorter hash fragment."
                />
              ) : null}
            </ConsolePanel>
          ) : (
            <ConsolePanel
              title="unspent outputs"
              subtitle="chain_utxos"
              icon="[]"
              padded={false}
              chip={
                <ConsolePill>
                  total {utxos?.total_value.toFixed(3) ?? '0.000'} units
                </ConsolePill>
              }
            >
              {utxos && utxos.utxos.length > 0 ? (
                <div className="overflow-x-auto">
                  <table className="crm-table">
                    <thead>
                      <tr>
                        <th>tx id</th>
                        <th>index</th>
                        <th>address</th>
                        <th className="text-right">value</th>
                      </tr>
                    </thead>
                    <tbody>
                      {utxos.utxos.map((utxo) => (
                        <tr key={`${utxo.tx_id}-${utxo.index}`}>
                          <td>{shortHash(utxo.tx_id, 12)}</td>
                          <td>{utxo.index}</td>
                          <td>{shortHash(utxo.address, 14)}</td>
                          <td className="text-right text-[var(--crm-accent)]">
                            {utxo.value}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <ConsoleEmpty title="no UTXOs returned" />
              )}
            </ConsolePanel>
          )}
        </>
      )}

      {currentBlock && !selectedTx ? (
        <>
          <ConsoleStatStrip columns={5}>
            <ConsoleStat
              label="height"
              value={`#${currentBlock.height.toLocaleString()}`}
              subtitle="confirmed"
              tone="accent"
            />
            <ConsoleStat
              label="txs"
              value={currentBlock.transactions.length}
              subtitle={`${currentBlock.transactions.filter((tx) => tx.is_coinbase).length} coinbase`}
            />
            <ConsoleStat
              label="size"
              value={`${(currentBlock.size_bytes / 1024).toFixed(2)} kB`}
              subtitle={`${currentBlock.size_bytes} bytes`}
            />
            <ConsoleStat
              label="nonce"
              value={currentBlock.nonce.toLocaleString()}
              subtitle="proof of work"
            />
            <ConsoleStat
              label="timestamp"
              value={formatRelativeTimestamp(currentBlock.timestamp)}
              subtitle={currentBlock.timestamp}
            />
          </ConsoleStatStrip>

          <div className="grid gap-3 xl:grid-cols-2">
            <ConsolePanel title="header" subtitle="block metadata" icon="[]">
              <ConsoleRow label="hash" value={currentBlock.hash} />
              <ConsoleRow label="prev_hash" value={currentBlock.prev_hash} />
              <ConsoleRow label="merkle_root" value={currentBlock.merkle_root} />
              <ConsoleRow label="nonce" value={currentBlock.nonce.toLocaleString()} />
              <ConsoleRow label="timestamp" value={currentBlock.timestamp} />
              <ConsoleRow label="size" value={`${currentBlock.size_bytes} bytes`} />
            </ConsolePanel>

            <ConsolePanel title="chain position" subtitle="adjacent linkage" icon="->">
              <div className="space-y-2">
                {blocks
                  .filter(
                    (block) =>
                      block.height <= currentBlock.height + 1 &&
                      block.height >= currentBlock.height - 2,
                  )
                  .sort((a, b) => b.height - a.height)
                  .map((block) => (
                    <button
                      key={block.height}
                      className={`w-full rounded-sm border px-4 py-3 text-left transition ${
                        block.height === currentBlock.height
                          ? 'border-[var(--crm-accent)] bg-[var(--crm-accent-bg)] text-[var(--crm-fg)]'
                          : 'border-[var(--crm-border)] bg-[var(--crm-panel-2)] text-[var(--crm-muted)]'
                      }`}
                      onClick={() => selectBlock(block)}
                      type="button"
                    >
                      <div className="flex items-center justify-between gap-3">
                        <span className="crm-mono">#{block.height}</span>
                        {block.height === currentBlock.height ? (
                          <ConsolePill tone="accent">current</ConsolePill>
                        ) : null}
                      </div>
                      <div className="mt-2 crm-mono text-xs">
                        {shortHash(block.hash, 14)}
                      </div>
                    </button>
                  ))}
              </div>
            </ConsolePanel>
          </div>

          <ConsolePanel
            title="transactions"
            subtitle={`${currentBlock.transactions.length} in this block`}
            icon="tx"
            padded={false}
          >
            <div className="overflow-x-auto">
              <table className="crm-table crm-table--interactive">
                <thead>
                  <tr>
                    <th>tx id</th>
                    <th>type</th>
                    <th>inputs</th>
                    <th>outputs</th>
                    <th className="text-right">size</th>
                  </tr>
                </thead>
                <tbody>
                  {currentBlock.transactions.map((tx) => (
                    <tr key={tx.id} onClick={() => setSelectedTx(tx)}>
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
                      <td className="text-right text-[var(--crm-dim)]">
                        {tx.size} bytes
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </ConsolePanel>
        </>
      ) : null}

      {currentBlock && selectedTx ? (
        <>
          <ConsolePanel
            title="transaction"
            subtitle="transaction_view"
            icon="tx"
            chip={
              <ConsolePill tone={selectedTx.is_coinbase ? 'accent' : 'neutral'}>
                {selectedTx.is_coinbase ? 'coinbase' : 'transfer'}
              </ConsolePill>
            }
          >
            <ConsoleRow label="id" value={selectedTx.id} />
            <ConsoleRow label="block" value={`#${currentBlock.height}`} />
            <ConsoleRow label="date" value={selectedTx.date} />
            <ConsoleRow label="size" value={`${selectedTx.size} bytes`} />
            <ConsoleRow label="message" value={selectedTx.message || '-'} mono={false} />
            <ConsoleRow
              label="total out"
              value={`${selectedTxTotal.toFixed(3)} units`}
            />
          </ConsolePanel>

          <div className="grid gap-3 xl:grid-cols-2">
            <ConsolePanel
              title="inputs"
              subtitle={`${selectedTx.inputs.length}`}
              icon="<-"
            >
              {selectedTx.inputs.length > 0 ? (
                <div className="space-y-3">
                  {selectedTx.inputs.map((input) => (
                    <div
                      key={`${input.prev_tx_id}-${input.output_index}`}
                      className="rounded-sm border border-dashed border-[var(--crm-border)] px-4 py-3"
                    >
                      <div className="crm-mono text-sm">
                        {shortHash(input.prev_tx_id, 14)}
                      </div>
                      <div className="mt-2 text-sm text-[var(--crm-dim)]">
                        output index {input.output_index}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <ConsoleEmpty
                  title="no inputs"
                  hint="Coinbase transactions mint protocol rewards and therefore have no spent inputs."
                />
              )}
            </ConsolePanel>

            <ConsolePanel
              title="outputs"
              subtitle={`${selectedTx.outputs.length}`}
              icon="->"
            >
              <div className="space-y-3">
                {selectedTx.outputs.map((output, index) => (
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
            </ConsolePanel>
          </div>
        </>
      ) : null}
    </div>
  );
}
