import { useState } from 'react';
import { Card, StatCard, Button } from '../components';
import { rpcClient } from '../services';
import { useWallet } from '../contexts';
import type { WalletSendResponse } from '../types';

export function Wallet() {
  const {
    wallets,
    activeWallet,
    setActiveWallet,
    addWallet,
    removeWallet,
    refreshWalletBalance,
  } = useWallet();

  // Add wallet form
  const [showAddForm, setShowAddForm] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [walletName, setWalletName] = useState('');
  const [keyPath, setKeyPath] = useState('');
  const [password, setPassword] = useState('');
  const [formError, setFormError] = useState<string | null>(null);
  const [formLoading, setFormLoading] = useState(false);

  // Send form
  const [sendTo, setSendTo] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [sendFee, setSendFee] = useState('1000');
  const [sendMessage, setSendMessage] = useState('');
  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);

  const handleAddWallet = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!walletName || !keyPath || !password) return;

    setFormLoading(true);
    setFormError(null);

    try {
      await addWallet(walletName, keyPath, password);
      setWalletName('');
      setKeyPath('');
      setPassword('');
      setShowAddForm(false);
    } catch (err) {
      setFormError(err instanceof Error ? err.message : 'Failed to add wallet');
    } finally {
      setFormLoading(false);
    }
  };

  const handleCreateWallet = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!walletName || !keyPath || !password) return;

    setFormLoading(true);
    setFormError(null);

    try {
      const result = await rpcClient.walletNew(password, keyPath);
      if (!result.success) {
        throw new Error('Failed to create wallet');
      }
      await addWallet(walletName, keyPath, password);
      setWalletName('');
      setKeyPath('');
      setPassword('');
      setShowCreateForm(false);
    } catch (err) {
      setFormError(err instanceof Error ? err.message : 'Failed to create wallet');
    } finally {
      setFormLoading(false);
    }
  };

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeWallet || !sendTo || !sendAmount) return;

    setSending(true);
    setSendError(null);

    try {
      const result: WalletSendResponse = await rpcClient.walletSend(
        { key_path: activeWallet.keyPath, password: activeWallet.password },
        sendTo,
        parseInt(sendAmount),
        sendFee ? parseInt(sendFee) : undefined,
        sendMessage || undefined
      );

      if (result.success) {
        alert(`Transaction sent! TX ID: ${result.tx_id}`);
        setSendTo('');
        setSendAmount('');
        setSendMessage('');
        refreshWalletBalance(activeWallet.name);
      } else {
        setSendError(result.error || 'Transaction failed');
      }
    } catch (err) {
      setSendError(err instanceof Error ? err.message : 'Failed to send');
    } finally {
      setSending(false);
    }
  };

  const closeAllForms = () => {
    setShowAddForm(false);
    setShowCreateForm(false);
    setFormError(null);
    setWalletName('');
    setKeyPath('');
    setPassword('');
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Wallet</h2>
        <div className="flex gap-2">
          <Button
            onClick={() => { closeAllForms(); setShowCreateForm(true); }}
            variant="primary"
          >
            + Create New
          </Button>
          <Button
            onClick={() => { closeAllForms(); setShowAddForm(true); }}
            variant="secondary"
          >
            + Import
          </Button>
        </div>
      </div>

      {/* Wallet Selector */}
      {wallets.length > 0 && (
        <Card>
          <div className="flex items-center gap-4">
            <label className="text-gray-400 text-sm font-medium whitespace-nowrap">
              Active Wallet:
            </label>
            <select
              value={activeWallet?.name || ''}
              onChange={(e) => {
                const wallet = wallets.find(w => w.name === e.target.value);
                setActiveWallet(wallet || null);
              }}
              className="flex-1 bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500 text-lg font-medium"
            >
              {wallets.map(w => (
                <option key={w.name} value={w.name}>
                  {w.name} ({w.balance?.balance ?? 0} units)
                </option>
              ))}
            </select>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => activeWallet && refreshWalletBalance(activeWallet.name)}
            >
              Refresh
            </Button>
            {activeWallet && (
              <Button
                variant="danger"
                size="sm"
                onClick={() => removeWallet(activeWallet.name)}
              >
                Remove
              </Button>
            )}
          </div>
        </Card>
      )}

      {/* Create Wallet Form */}
      {showCreateForm && (
        <Card title="Create New Wallet">
          <form onSubmit={handleCreateWallet} className="space-y-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">Wallet Name</label>
              <input
                type="text"
                value={walletName}
                onChange={(e) => setWalletName(e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                placeholder="My Wallet"
                required
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Keystore Path</label>
              <input
                type="text"
                value={keyPath}
                onChange={(e) => setKeyPath(e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                placeholder="keys/new_wallet.json"
                required
              />
              <p className="text-xs text-gray-500 mt-1">Path where the keystore file will be saved</p>
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Password</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                placeholder="Choose a strong password"
                required
              />
            </div>
            {formError && <p className="text-red-400 text-sm">{formError}</p>}
            <div className="flex gap-2">
              <Button type="submit" loading={formLoading}>Create Wallet</Button>
              <Button type="button" variant="secondary" onClick={closeAllForms}>
                Cancel
              </Button>
            </div>
          </form>
        </Card>
      )}

      {/* Import Wallet Form */}
      {showAddForm && (
        <Card title="Import Existing Wallet">
          <form onSubmit={handleAddWallet} className="space-y-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">Wallet Name</label>
              <input
                type="text"
                value={walletName}
                onChange={(e) => setWalletName(e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                placeholder="My Wallet"
                required
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Keystore Path</label>
              <input
                type="text"
                value={keyPath}
                onChange={(e) => setKeyPath(e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                placeholder="keys/miner_wallet.json"
                required
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Password</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                placeholder="Enter password"
                required
              />
            </div>
            {formError && <p className="text-red-400 text-sm">{formError}</p>}
            <div className="flex gap-2">
              <Button type="submit" loading={formLoading}>Import Wallet</Button>
              <Button type="button" variant="secondary" onClick={closeAllForms}>
                Cancel
              </Button>
            </div>
          </form>
        </Card>
      )}

      {/* No Wallets */}
      {wallets.length === 0 && !showAddForm && !showCreateForm && (
        <Card>
          <div className="text-center py-12">
            <p className="text-gray-400 mb-4">No wallets loaded</p>
            <p className="text-gray-500 text-sm">
              Create a new wallet or import an existing one to get started.
            </p>
          </div>
        </Card>
      )}

      {/* Active Wallet Details */}
      {activeWallet && (
        <>
          {/* Stats */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <StatCard
              icon="$"
              label="Balance"
              value={`${activeWallet.balance?.balance ?? 0} units`}
            />
            <StatCard
              icon="#"
              label="UTXOs"
              value={activeWallet.balance?.utxo_count ?? 0}
            />
            <StatCard
              icon="*"
              label="Wallet"
              value={activeWallet.name}
            />
          </div>

          {/* Address */}
          <Card title="Your Address">
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-gray-700 px-3 py-2 rounded font-mono text-sm break-all">
                {activeWallet.address}
              </code>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  navigator.clipboard.writeText(activeWallet.address);
                  alert('Address copied!');
                }}
              >
                Copy
              </Button>
            </div>
          </Card>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Send Form */}
            <Card title="Send Transaction">
              <form onSubmit={handleSend} className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Recipient Address</label>
                  <input
                    type="text"
                    value={sendTo}
                    onChange={(e) => setSendTo(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500 font-mono text-sm"
                    placeholder="Enter recipient address"
                    required
                  />
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm text-gray-400 mb-1">Amount</label>
                    <input
                      type="number"
                      value={sendAmount}
                      onChange={(e) => setSendAmount(e.target.value)}
                      className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                      placeholder="0"
                      required
                      min="1"
                    />
                  </div>
                  <div>
                    <label className="block text-sm text-gray-400 mb-1">Fee</label>
                    <input
                      type="number"
                      value={sendFee}
                      onChange={(e) => setSendFee(e.target.value)}
                      className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                      placeholder="1000"
                      min="0"
                    />
                  </div>
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Message (optional)</label>
                  <input
                    type="text"
                    value={sendMessage}
                    onChange={(e) => setSendMessage(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                    placeholder="Optional message"
                  />
                </div>
                {sendError && <p className="text-red-400 text-sm">{sendError}</p>}
                <Button type="submit" loading={sending} className="w-full">
                  Send Transaction
                </Button>
              </form>
            </Card>

            {/* UTXOs */}
            <Card title="Your UTXOs">
              <div className="space-y-2 max-h-80 overflow-y-auto">
                {activeWallet.balance?.utxos && activeWallet.balance.utxos.length > 0 ? (
                  activeWallet.balance.utxos.map((utxo, idx) => (
                    <div key={idx} className="p-3 bg-gray-700 rounded text-sm">
                      <div className="flex justify-between items-center">
                        <span className="text-gray-400">#{utxo.index}</span>
                        <span className="text-green-400 font-bold">{utxo.value} units</span>
                      </div>
                      <div className="font-mono text-xs text-gray-400 truncate mt-1">
                        {utxo.tx_id}
                      </div>
                    </div>
                  ))
                ) : (
                  <p className="text-gray-400 text-center py-4">No UTXOs available</p>
                )}
              </div>
            </Card>
          </div>
        </>
      )}
    </div>
  );
}
