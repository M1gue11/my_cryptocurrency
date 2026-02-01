// Chain Types
// Mirrors: project/src/daemon/types/chain.rs

import type { TransactionViewResponse } from './transaction';

export interface ChainStatusResponse {
  block_count: number;
  is_valid: boolean;
  last_block_hash?: string;
  last_block_date?: string;
}

export interface BlockInfo {
  height: number;
  hash: string;
  prev_hash: string;
  merkle_root: string;
  nonce: number;
  timestamp: string;
  transactions: TransactionViewResponse[];
  size_bytes: number;
}

export interface ChainShowResponse {
  blocks: BlockInfo[];
}

export interface UtxoInfo {
  tx_id: string;
  index: number;
  value: number;
  address: string;
}

export interface UtxosParams {
  limit?: number;
}

export interface UtxosResponse {
  utxos: UtxoInfo[];
  total_value: number;
}
