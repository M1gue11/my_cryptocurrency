// Mining Types
// Mirrors: project/src/daemon/types/mine.rs

import type { TransactionViewResponse } from './transaction';

export interface MineBlockResponse {
  success: boolean;
  transactions: TransactionViewResponse[];
  block_hash?: string;
  nonce?: number;
  error?: string;
}
