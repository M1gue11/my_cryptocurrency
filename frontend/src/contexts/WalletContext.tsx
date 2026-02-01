import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import { rpcClient } from '../services';
import type { WalletBalanceResponse } from '../types';

export interface LoadedWallet {
  name: string;
  keyPath: string;
  password: string;
  address: string;
  balance: WalletBalanceResponse | null;
}

interface WalletContextType {
  wallets: LoadedWallet[];
  activeWallet: LoadedWallet | null;
  setActiveWallet: (wallet: LoadedWallet | null) => void;
  addWallet: (name: string, keyPath: string, password: string) => Promise<void>;
  removeWallet: (name: string) => void;
  refreshWalletBalance: (name: string) => Promise<void>;
  refreshAllBalances: () => Promise<void>;
  loading: boolean;
  error: string | null;
}

const WalletContext = createContext<WalletContextType | null>(null);

export function WalletProvider({ children }: { children: ReactNode }) {
  const [wallets, setWallets] = useState<LoadedWallet[]>([]);
  const [activeWallet, setActiveWallet] = useState<LoadedWallet | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const addWallet = useCallback(async (name: string, keyPath: string, password: string) => {
    // Check if wallet with this name already exists
    if (wallets.some(w => w.name === name)) {
      throw new Error(`Wallet with name "${name}" already loaded`);
    }

    setLoading(true);
    setError(null);

    try {
      // Get address and balance
      const [addrRes, balRes] = await Promise.all([
        rpcClient.walletAddress(keyPath, password),
        rpcClient.walletBalance(keyPath, password),
      ]);

      const newWallet: LoadedWallet = {
        name,
        keyPath,
        password,
        address: addrRes.address,
        balance: balRes,
      };

      setWallets(prev => [...prev, newWallet]);

      // If this is the first wallet, make it active
      if (wallets.length === 0) {
        setActiveWallet(newWallet);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load wallet';
      setError(message);
      throw new Error(message);
    } finally {
      setLoading(false);
    }
  }, [wallets]);

  const removeWallet = useCallback((name: string) => {
    setWallets(prev => prev.filter(w => w.name !== name));

    // If active wallet was removed, switch to first available or null
    if (activeWallet?.name === name) {
      setWallets(prev => {
        const remaining = prev.filter(w => w.name !== name);
        setActiveWallet(remaining[0] || null);
        return remaining;
      });
    }
  }, [activeWallet]);

  const refreshWalletBalance = useCallback(async (name: string) => {
    const wallet = wallets.find(w => w.name === name);
    if (!wallet) return;

    try {
      const balRes = await rpcClient.walletBalance(wallet.keyPath, wallet.password);

      setWallets(prev => prev.map(w =>
        w.name === name ? { ...w, balance: balRes } : w
      ));

      // Update active wallet if it's the one being refreshed
      if (activeWallet?.name === name) {
        setActiveWallet(prev => prev ? { ...prev, balance: balRes } : null);
      }
    } catch (err) {
      console.error('Failed to refresh balance:', err);
    }
  }, [wallets, activeWallet]);

  const refreshAllBalances = useCallback(async () => {
    setLoading(true);
    try {
      await Promise.all(wallets.map(w => refreshWalletBalance(w.name)));
    } finally {
      setLoading(false);
    }
  }, [wallets, refreshWalletBalance]);

  return (
    <WalletContext.Provider
      value={{
        wallets,
        activeWallet,
        setActiveWallet,
        addWallet,
        removeWallet,
        refreshWalletBalance,
        refreshAllBalances,
        loading,
        error,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}
