// Mining Types
// Mirrors: project/src/daemon/types/mine.rs

import type { TransactionViewResponse } from './transaction';

export interface MineBlockResponse {
  success: boolean;
  transactions: TransactionViewResponse[];
  block_hash?: string;
  nonce?: number;
  error?: string;
  target?: string;
  next_target?: string;
  next_difficulty?: string;
}

export interface MiningInfoResponse {
  keep_mining_enabled: boolean;
  is_currently_mining: boolean;
  started_at?: string | null;
  last_mined_block?: MineBlockResponse | null;
}

export interface KeepMiningResponse {
  success: boolean;
}
