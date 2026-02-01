// Node Types
// Mirrors: project/src/daemon/types/node.rs

import type { TransactionViewResponse } from './transaction';

export interface NodeStatusResponse {
  version: string;
  peers_connected: number;
  block_height: number;
  top_block_hash: string;
}

export interface MempoolEntry {
  tx: TransactionViewResponse;
}

export interface MempoolResponse {
  count: number;
  transactions: MempoolEntry[];
}
