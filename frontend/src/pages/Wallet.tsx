import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useWallet } from '../contexts';
import {
  ConsoleButton,
  ConsoleEmpty,
  ConsolePageHeader,
  ConsolePanel,
  ConsolePill,
  ConsoleRow,
  ConsoleTabs,
  shortHash,
} from '../components';
import { rpcClient } from '../services';
import type { GeneratedKey, WalletSendResponse } from '../types';

type WalletTab = 'overview' | 'send' | 'receive' | 'utxos' | 'keys';
type KeyPurpose = 'receive' | 'change';

export function Wallet() {
  const [searchParams, setSearchParams] = useSearchParams();
  const {
    wallets,
    activeWallet,
    setActiveWallet,
    addWallet,
    removeWallet,
    refreshWalletBalance,
  } = useWallet();

  const [showImportForm, setShowImportForm] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [walletName, setWalletName] = useState('');
  const [keyPath, setKeyPath] = useState('');
  const [password, setPassword] = useState('');
  const [formError, setFormError] = useState<string | null>(null);
  const [formLoading, setFormLoading] = useState(false);

  const [sendTo, setSendTo] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [sendFee, setSendFee] = useState('1000');
  const [sendMessage, setSendMessage] = useState('');
  const [sending, setSending] = useState(false);
  const [sendFeedback, setSendFeedback] = useState<{
    tone: 'good' | 'warn';
    message: string;
  } | null>(null);

  const [generatedKeys, setGeneratedKeys] = useState<GeneratedKey[]>([]);
  const [keyPurpose, setKeyPurpose] = useState<KeyPurpose>('receive');
  const [keyCount, setKeyCount] = useState('1');
  const [generatingKeys, setGeneratingKeys] = useState(false);
  const [keyFeedback, setKeyFeedback] = useState<string | null>(null);
  const [copiedLabel, setCopiedLabel] = useState<string | null>(null);

  const tabParam = searchParams.get('tab') as WalletTab | null;
  const activeTab: WalletTab =
    tabParam && ['overview', 'send', 'receive', 'utxos', 'keys'].includes(tabParam)
      ? tabParam
      : 'overview';

  useEffect(() => {
    if (!searchParams.get('tab')) {
      setSearchParams({ tab: 'overview' }, { replace: true });
    }
  }, [searchParams, setSearchParams]);

  const balance = activeWallet?.balance?.balance ?? 0;
  const utxos = activeWallet?.balance?.utxos ?? [];

  const closeForms = () => {
    setShowImportForm(false);
    setShowCreateForm(false);
    setFormError(null);
    setWalletName('');
    setKeyPath('');
    setPassword('');
  };

  const switchTab = (tab: WalletTab) => {
    setSearchParams({ tab });
  };

  const handleImportWallet = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!walletName || !keyPath || !password) return;

    try {
      setFormLoading(true);
      setFormError(null);
      await addWallet(walletName, keyPath, password);
      closeForms();
    } catch (nextError) {
      setFormError(
        nextError instanceof Error ? nextError.message : 'Failed to import wallet',
      );
    } finally {
      setFormLoading(false);
    }
  };

  const handleCreateWallet = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!walletName || !keyPath || !password) return;

    try {
      setFormLoading(true);
      setFormError(null);
      const result = await rpcClient.walletNew(password, keyPath);
      if (!result.success) {
        throw new Error('Failed to create wallet');
      }
      await addWallet(walletName, keyPath, password);
      closeForms();
    } catch (nextError) {
      setFormError(
        nextError instanceof Error ? nextError.message : 'Failed to create wallet',
      );
    } finally {
      setFormLoading(false);
    }
  };

  const handleSend = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!activeWallet || !sendTo || !sendAmount) return;

    try {
      setSending(true);
      setSendFeedback(null);

      const response: WalletSendResponse = await rpcClient.walletSend(
        {
          key_path: activeWallet.keyPath,
          password: activeWallet.password,
        },
        sendTo,
        Number(sendAmount),
        sendFee ? Number(sendFee) : undefined,
        sendMessage || undefined,
      );

      if (!response.success) {
        setSendFeedback({
          tone: 'warn',
          message: response.error ?? 'Transaction failed',
        });
        return;
      }

      setSendFeedback({
        tone: 'good',
        message: `Submitted to mempool: ${response.tx_id}`,
      });
      setSendTo('');
      setSendAmount('');
      setSendMessage('');
      await refreshWalletBalance(activeWallet.name);
    } catch (nextError) {
      setSendFeedback({
        tone: 'warn',
        message: nextError instanceof Error ? nextError.message : 'Failed to send',
      });
    } finally {
      setSending(false);
    }
  };

  const handleGenerateKeys = async (countOverride?: number, purposeOverride?: KeyPurpose) => {
    if (!activeWallet) return;

    try {
      setGeneratingKeys(true);
      setKeyFeedback(null);

      const count = (countOverride ?? Number(keyCount)) || 1;
      const purpose = purposeOverride ?? keyPurpose;
      const response = await rpcClient.walletGenerateKeys(
        {
          key_path: activeWallet.keyPath,
          password: activeWallet.password,
        },
        count,
        purpose === 'change' ? 1 : 0,
      );

      setGeneratedKeys(response.keys);
      setKeyFeedback(`Generated ${response.keys.length} ${purpose} key(s)`);
    } catch (nextError) {
      setKeyFeedback(
        nextError instanceof Error ? nextError.message : 'Failed to derive keys',
      );
    } finally {
      setGeneratingKeys(false);
    }
  };

  const handleCopy = async (value: string, label: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopiedLabel(label);
      setTimeout(() => setCopiedLabel(null), 1600);
    } catch {
      setCopiedLabel(`Unable to copy ${label}`);
      setTimeout(() => setCopiedLabel(null), 1600);
    }
  };

  const totalDebit = useMemo(
    () => Number(sendAmount || 0) + Number(sendFee || 0),
    [sendAmount, sendFee],
  );

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow="wallet_balance . wallet_send . wallet_generate_keys"
        title="Wallet"
        actions={
          <>
            {activeWallet ? (
              <ConsolePill tone="good">
                session loaded . {activeWallet.name}
              </ConsolePill>
            ) : (
              <ConsolePill tone="warn">no active wallet</ConsolePill>
            )}
            <ConsoleButton
              tone="primary"
              onClick={() => {
                closeForms();
                setShowCreateForm(true);
              }}
            >
              create new
            </ConsoleButton>
            <ConsoleButton
              onClick={() => {
                closeForms();
                setShowImportForm(true);
              }}
            >
              import
            </ConsoleButton>
          </>
        }
      />

      {copiedLabel ? (
        <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] px-4 py-3 text-sm text-[var(--crm-muted)]">
          {copiedLabel}
        </div>
      ) : null}

      {wallets.length > 0 ? (
        <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-4">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
            <div className="min-w-[220px]">
              <div className="crm-field-label">active wallet</div>
              <select
                className="crm-select"
                value={activeWallet?.name ?? ''}
                onChange={(event) => {
                  const nextWallet = wallets.find(
                    (wallet) => wallet.name === event.target.value,
                  );
                  setActiveWallet(nextWallet ?? null);
                }}
              >
                {wallets.map((wallet) => (
                  <option key={wallet.name} value={wallet.name}>
                    {wallet.name}
                  </option>
                ))}
              </select>
            </div>
            <div className="text-sm text-[var(--crm-dim)]">
              Loaded wallets are kept only in this browser session. The backend
              still expects keystore path and password on each RPC call.
            </div>
            <div className="ml-auto flex flex-wrap gap-2">
              <ConsoleButton
                onClick={() => activeWallet && void refreshWalletBalance(activeWallet.name)}
                disabled={!activeWallet}
              >
                refresh
              </ConsoleButton>
              <ConsoleButton
                tone="danger"
                onClick={() => activeWallet && removeWallet(activeWallet.name)}
                disabled={!activeWallet}
              >
                remove
              </ConsoleButton>
            </div>
          </div>
        </div>
      ) : null}

      {showCreateForm || showImportForm ? (
        <div className="grid gap-3 xl:grid-cols-2">
          {showCreateForm ? (
            <WalletFormPanel
              title="create new wallet"
              subtitle="wallet_new"
              actionLabel="create wallet"
              loading={formLoading}
              error={formError}
              walletName={walletName}
              keyPath={keyPath}
              password={password}
              onWalletNameChange={setWalletName}
              onKeyPathChange={setKeyPath}
              onPasswordChange={setPassword}
              onCancel={closeForms}
              onSubmit={handleCreateWallet}
              keyPathHint="Where the keystore JSON should be written"
            />
          ) : null}
          {showImportForm ? (
            <WalletFormPanel
              title="import existing wallet"
              subtitle="wallet_import"
              actionLabel="import wallet"
              loading={formLoading}
              error={formError}
              walletName={walletName}
              keyPath={keyPath}
              password={password}
              onWalletNameChange={setWalletName}
              onKeyPathChange={setKeyPath}
              onPasswordChange={setPassword}
              onCancel={closeForms}
              onSubmit={handleImportWallet}
              keyPathHint="Path to an existing keystore JSON file"
            />
          ) : null}
        </div>
      ) : null}

      {activeWallet ? (
        <>
          <div className="grid gap-4 rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] px-5 py-4 lg:grid-cols-3">
            <div>
              <div className="crm-field-label">active keystore</div>
              <div className="crm-mono text-sm">{activeWallet.keyPath}</div>
              <div className="mt-1 text-sm text-[var(--crm-dim)]">
                imported session . loaded in browser memory
              </div>
            </div>
            <div>
              <div className="crm-field-label">primary address</div>
              <div className="crm-mono text-sm">{activeWallet.address}</div>
              <div className="mt-1 text-sm text-[var(--crm-dim)]">
                receive . copy . key derivation available
              </div>
            </div>
            <div className="text-left lg:text-right">
              <div className="crm-field-label">balance</div>
              <div className="crm-mono text-3xl tracking-[-0.04em] text-[var(--crm-accent)]">
                {balance.toFixed(3)}{' '}
                <span className="text-sm text-[var(--crm-dim)]">units</span>
              </div>
              <div className="mt-1 text-sm text-[var(--crm-dim)]">
                {activeWallet.balance?.utxo_count ?? 0} UTXOs
              </div>
            </div>
          </div>

          <ConsoleTabs
            active={activeTab}
            onChange={switchTab}
            items={[
              { key: 'overview', label: 'Overview' },
              { key: 'send', label: 'Send' },
              { key: 'receive', label: 'Receive' },
              { key: 'utxos', label: 'UTXOs' },
              { key: 'keys', label: 'Keys & Session' },
            ]}
          />

          {activeTab === 'overview' ? (
            <div className="grid gap-3 xl:grid-cols-[1.25fr_1fr]">
              <ConsolePanel title="wallet holdings" subtitle="active UTXOs" icon="[]">
                {utxos.length > 0 ? (
                  <div className="space-y-3">
                    {utxos.slice(0, 5).map((utxo) => (
                      <div
                        key={`${utxo.tx_id}-${utxo.index}`}
                        className="rounded-sm border border-dashed border-[var(--crm-border)] px-4 py-3"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <div className="crm-mono text-sm">
                            {shortHash(utxo.tx_id, 12)}:{utxo.index}
                          </div>
                          <div className="crm-mono text-sm text-[var(--crm-accent)]">
                            {utxo.value} units
                          </div>
                        </div>
                        <div className="mt-2 text-sm text-[var(--crm-dim)]">
                          {shortHash(utxo.address, 18)}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <ConsoleEmpty
                    title="no spendable UTXOs"
                    hint="Mine or receive funds before trying to create transactions."
                  />
                )}
              </ConsolePanel>

              <ConsolePanel title="quick actions" subtitle="operator shortcuts" icon="*">
                <div className="grid gap-2 sm:grid-cols-2">
                  <ConsoleButton tone="primary" onClick={() => switchTab('send')}>
                    send
                  </ConsoleButton>
                  <ConsoleButton onClick={() => switchTab('receive')}>
                    receive
                  </ConsoleButton>
                  <ConsoleButton onClick={() => switchTab('utxos')}>
                    review UTXOs
                  </ConsoleButton>
                  <ConsoleButton onClick={() => switchTab('keys')}>
                    derive keys
                  </ConsoleButton>
                </div>
                <div className="mt-4 rounded-sm border border-dashed border-[var(--crm-border)] bg-[var(--crm-panel-2)] px-4 py-3 text-sm text-[var(--crm-dim)]">
                  This UI keeps your loaded wallet session in memory so the path
                  and password can be reused for RPC calls. Removing the wallet
                  clears that session from the frontend.
                </div>
              </ConsolePanel>
            </div>
          ) : null}

          {activeTab === 'send' ? (
            <div className="grid gap-3 xl:grid-cols-[1.3fr_1fr]">
              <ConsolePanel title="send units" subtitle="wallet_send" icon="->">
                <form className="space-y-4" onSubmit={handleSend}>
                  <Field label="destination address">
                    <input
                      className="crm-input"
                      value={sendTo}
                      onChange={(event) => setSendTo(event.target.value)}
                      placeholder="crm1..."
                    />
                  </Field>

                  <div className="grid gap-4 sm:grid-cols-2">
                    <Field label="amount">
                      <input
                        className="crm-input"
                        type="number"
                        min="0"
                        step="1"
                        value={sendAmount}
                        onChange={(event) => setSendAmount(event.target.value)}
                        placeholder="0"
                      />
                    </Field>
                    <Field label="fee">
                      <input
                        className="crm-input"
                        type="number"
                        min="0"
                        step="1"
                        value={sendFee}
                        onChange={(event) => setSendFee(event.target.value)}
                      />
                    </Field>
                  </div>

                  <Field label="message (optional)">
                    <input
                      className="crm-input"
                      value={sendMessage}
                      onChange={(event) => setSendMessage(event.target.value)}
                      placeholder="Optional memo"
                    />
                  </Field>

                  <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel-2)] px-4 py-3">
                    <ConsoleRow label="available" value={`${balance.toFixed(3)} units`} />
                    <ConsoleRow
                      label="total debited"
                      value={`${totalDebit.toFixed(3)} units`}
                    />
                    <ConsoleRow
                      label="remaining"
                      value={`${Math.max(balance - totalDebit, 0).toFixed(3)} units`}
                    />
                  </div>

                  <div className="flex flex-wrap gap-2">
                    <ConsoleButton
                      tone="primary"
                      type="submit"
                      loading={sending}
                      disabled={!sendTo || !sendAmount}
                    >
                      review & send
                    </ConsoleButton>
                    <ConsoleButton
                      type="button"
                      onClick={() => {
                        setSendTo('');
                        setSendAmount('');
                        setSendFee('1000');
                        setSendMessage('');
                        setSendFeedback(null);
                      }}
                    >
                      clear
                    </ConsoleButton>
                  </div>
                </form>

                {sendFeedback ? (
                  <div
                    className={`mt-4 rounded-sm border px-4 py-3 text-sm ${
                      sendFeedback.tone === 'good'
                        ? 'border-[var(--crm-good)]/30 bg-[var(--crm-good-bg)] text-[var(--crm-good)]'
                        : 'border-[var(--crm-warn)]/30 bg-[var(--crm-warn-bg)] text-[var(--crm-warn)]'
                    }`}
                  >
                    {sendFeedback.message}
                  </div>
                ) : null}
              </ConsolePanel>

              <ConsolePanel title="coin selection" subtitle="wallet spends UTXOs" icon="[]">
                {utxos.length > 0 ? (
                  <div className="space-y-3">
                    {utxos.slice(0, 4).map((utxo) => (
                      <div
                        key={`${utxo.tx_id}-${utxo.index}`}
                        className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel-2)] px-4 py-3"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <div className="crm-mono text-sm">
                            {shortHash(utxo.tx_id, 12)}
                          </div>
                          <div className="crm-mono text-sm text-[var(--crm-accent)]">
                            {utxo.value}
                          </div>
                        </div>
                        <div className="mt-2 text-sm text-[var(--crm-dim)]">
                          index {utxo.index} . {shortHash(utxo.address, 16)}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <ConsoleEmpty title="no available inputs" />
                )}
              </ConsolePanel>
            </div>
          ) : null}

          {activeTab === 'receive' ? (
            <div className="grid gap-3 xl:grid-cols-[0.95fr_1.25fr]">
              <ConsolePanel title="current receiving address" subtitle="wallet_address" icon="<-">
                <div className="mx-auto grid h-40 w-40 place-items-center rounded-sm border border-[var(--crm-border)] bg-[linear-gradient(135deg,var(--crm-panel-2),var(--crm-bg-2))]">
                  <div className="crm-logo-mark h-10 w-10" />
                </div>
                <div className="mt-4 text-center">
                  <div className="crm-mono break-all text-sm">{activeWallet.address}</div>
                  <div className="mt-3 flex flex-wrap justify-center gap-2">
                    <ConsoleButton
                      size="sm"
                      onClick={() => void handleCopy(activeWallet.address, 'address copied')}
                    >
                      copy
                    </ConsoleButton>
                    <ConsoleButton
                      size="sm"
                      onClick={() => void handleGenerateKeys(1, 'receive')}
                      loading={generatingKeys}
                    >
                      new address
                    </ConsoleButton>
                  </div>
                </div>
              </ConsolePanel>

              <ConsolePanel
                title="derived addresses"
                subtitle="wallet_generate_keys"
                icon="#"
                padded={false}
              >
                {generatedKeys.length > 0 ? (
                  <div className="overflow-x-auto">
                    <table className="crm-table">
                      <thead>
                        <tr>
                          <th>index</th>
                          <th>address</th>
                          <th>public key</th>
                        </tr>
                      </thead>
                      <tbody>
                        {generatedKeys.map((key, index) => (
                          <tr key={`${key.address}-${index}`}>
                            <td className="text-[var(--crm-accent)]">{index}</td>
                            <td>{shortHash(key.address, 18)}</td>
                            <td>{shortHash(key.public_key, 18)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <ConsoleEmpty
                    title="no derived keys loaded"
                    hint="Use the new address action or the keys tab to derive receive or change addresses from the active keystore."
                  />
                )}
              </ConsolePanel>
            </div>
          ) : null}

          {activeTab === 'utxos' ? (
            <ConsolePanel
              title="your UTXOs"
              subtitle="wallet_balance"
              icon="[]"
              padded={false}
              chip={<ConsolePill>total {balance.toFixed(3)} units</ConsolePill>}
            >
              {utxos.length > 0 ? (
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
                      {utxos.map((utxo) => (
                        <tr key={`${utxo.tx_id}-${utxo.index}`}>
                          <td>{shortHash(utxo.tx_id, 14)}</td>
                          <td>{utxo.index}</td>
                          <td>{shortHash(utxo.address, 18)}</td>
                          <td className="text-right text-[var(--crm-accent)]">
                            {utxo.value}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <ConsoleEmpty
                  title="no UTXOs available"
                  hint="Once this wallet receives funds, spendable outputs will be listed here."
                />
              )}
            </ConsolePanel>
          ) : null}

          {activeTab === 'keys' ? (
            <div className="grid gap-3 xl:grid-cols-2">
              <ConsolePanel title="keystore session" subtitle="active" icon="[]">
                <ConsoleRow label="wallet" value={activeWallet.name} />
                <ConsoleRow label="file" value={activeWallet.keyPath} />
                <ConsoleRow label="address" value={activeWallet.address} />
                <ConsoleRow label="balance" value={`${balance.toFixed(3)} units`} />
                <div className="mt-4 rounded-sm border border-dashed border-[var(--crm-border)] bg-[var(--crm-panel-2)] px-4 py-3 text-sm text-[var(--crm-dim)]">
                  The backend does not maintain a wallet session. This frontend
                  stores the active wallet credentials in memory so you do not
                  need to re-enter them for every RPC call.
                </div>
              </ConsolePanel>

              <ConsolePanel title="generate new keys" subtitle="wallet_generate_keys" icon="*">
                <div className="space-y-4">
                  <Field label="purpose">
                    <div className="flex flex-wrap gap-2">
                      {(['receive', 'change'] as KeyPurpose[]).map((purpose) => (
                        <button
                          key={purpose}
                          className={`rounded-sm border px-4 py-2 crm-mono text-xs uppercase tracking-[0.08em] ${
                            keyPurpose === purpose
                              ? 'border-[var(--crm-accent)] bg-[var(--crm-accent-bg)] text-[var(--crm-accent)]'
                              : 'border-[var(--crm-border)] bg-[var(--crm-panel-2)] text-[var(--crm-muted)]'
                          }`}
                          onClick={() => setKeyPurpose(purpose)}
                          type="button"
                        >
                          {purpose}
                        </button>
                      ))}
                    </div>
                  </Field>

                  <Field label="count">
                    <input
                      className="crm-input"
                      type="number"
                      min="1"
                      max="20"
                      value={keyCount}
                      onChange={(event) => setKeyCount(event.target.value)}
                    />
                  </Field>

                  <ConsoleButton
                    tone="primary"
                    onClick={() => void handleGenerateKeys()}
                    loading={generatingKeys}
                  >
                    derive key(s)
                  </ConsoleButton>

                  {keyFeedback ? (
                    <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel-2)] px-4 py-3 text-sm text-[var(--crm-muted)]">
                      {keyFeedback}
                    </div>
                  ) : null}

                  {generatedKeys.length > 0 ? (
                    <div className="space-y-2">
                      {generatedKeys.map((key, index) => (
                        <div
                          key={`${key.address}-${index}`}
                          className="rounded-sm border border-dashed border-[var(--crm-border)] px-4 py-3"
                        >
                          <div className="crm-mono text-sm">
                            {shortHash(key.address, 18)}
                          </div>
                          <div className="mt-1 text-xs text-[var(--crm-dim)]">
                            {shortHash(key.public_key, 18)}
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : null}
                </div>
              </ConsolePanel>
            </div>
          ) : null}
        </>
      ) : (
        <ConsoleEmpty
          title="no wallets loaded"
          hint="Create a new wallet or import an existing keystore to unlock wallet operations."
          action={
            <div className="flex flex-wrap justify-center gap-2">
              <ConsoleButton
                tone="primary"
                onClick={() => {
                  closeForms();
                  setShowCreateForm(true);
                }}
              >
                create wallet
              </ConsoleButton>
              <ConsoleButton
                onClick={() => {
                  closeForms();
                  setShowImportForm(true);
                }}
              >
                import wallet
              </ConsoleButton>
            </div>
          }
        />
      )}
    </div>
  );
}

interface WalletFormPanelProps {
  title: string;
  subtitle: string;
  actionLabel: string;
  loading: boolean;
  error: string | null;
  walletName: string;
  keyPath: string;
  password: string;
  keyPathHint: string;
  onWalletNameChange: (value: string) => void;
  onKeyPathChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: (event: React.FormEvent) => void;
}

function WalletFormPanel({
  title,
  subtitle,
  actionLabel,
  loading,
  error,
  walletName,
  keyPath,
  password,
  keyPathHint,
  onWalletNameChange,
  onKeyPathChange,
  onPasswordChange,
  onCancel,
  onSubmit,
}: WalletFormPanelProps) {
  return (
    <ConsolePanel title={title} subtitle={subtitle} icon="*">
      <form className="space-y-4" onSubmit={onSubmit}>
        <Field label="wallet name">
          <input
            className="crm-input"
            value={walletName}
            onChange={(event) => onWalletNameChange(event.target.value)}
            placeholder="demo-wallet"
            required
          />
        </Field>
        <Field label="keystore path" hint={keyPathHint}>
          <input
            className="crm-input"
            value={keyPath}
            onChange={(event) => onKeyPathChange(event.target.value)}
            placeholder="keys/my_wallet.json"
            required
          />
        </Field>
        <Field label="password">
          <input
            className="crm-input"
            type="password"
            value={password}
            onChange={(event) => onPasswordChange(event.target.value)}
            placeholder="Enter password"
            required
          />
        </Field>
        {error ? (
          <div className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
            {error}
          </div>
        ) : null}
        <div className="flex flex-wrap gap-2">
          <ConsoleButton tone="primary" type="submit" loading={loading}>
            {actionLabel}
          </ConsoleButton>
          <ConsoleButton type="button" onClick={onCancel}>
            cancel
          </ConsoleButton>
        </div>
      </form>
    </ConsolePanel>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div>
      <div className="crm-field-label">
        {label}
        {hint ? <span className="ml-2 text-[var(--crm-faint)] normal-case">{hint}</span> : null}
      </div>
      {children}
    </div>
  );
}
