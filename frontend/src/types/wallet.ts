// Wallet Types
// Mirrors: project/src/daemon/types/wallet.rs

import type { UtxoInfo } from './chain';

export interface WalletAccessParams {
  key_path: string;
  password: string;
}

export interface WalletNewParams {
  password: string;
  path?: string;
}

export interface WalletNewResponse {
  success: boolean;
  address?: string;
  is_imported_wallet: boolean;
}

export interface WalletInfo {
  name: string;
  balance: number;
}

export interface WalletListResponse {
  wallets: WalletInfo[];
}

export interface WalletAddressParams {
  key_path: string;
  password: string;
}

export interface WalletAddressResponse {
  address: string;
}

export interface WalletBalanceParams {
  key_path: string;
  password: string;
}

export interface WalletBalanceResponse {
  balance: number;
  utxo_count: number;
  utxos: UtxoInfo[];
}

export interface WalletSendParams {
  from: WalletAccessParams;
  to: string;
  amount: number;
  fee?: number;
  message?: string;
}

export interface WalletSendResponse {
  success: boolean;
  tx_id?: string;
  error?: string;
}

export interface WalletGenerateKeysParams {
  wallet: WalletAccessParams;
  count?: number;
  derivation_type?: number; // 0 = receive, 1 = change
}

export interface GeneratedKey {
  address: string;
  public_key: string;
}

export interface WalletGenerateKeysResponse {
  keys: GeneratedKey[];
}
