// Mining Types
// Mirrors: project/src/daemon/types/mine.rs

import type { TransactionViewResponse } from './transaction';

export interface MineBlockResponse {
  success: boolean;
  transactions: TransactionViewResponse[];
  block_hash?: string;
  nonce?: number;
  error?: string;
  difficulty?: number;
  next_difficulty?: number;
}

// Returned by mine_info: ISO datetime string if currently mining, null otherwise
export type MiningInfoResponse = string | null;

export interface KeepMiningResponse {
  success: boolean;
}
