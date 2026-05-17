// Node Types
// Mirrors: project/src/daemon/types/node.rs

import type { TransactionViewResponse } from './transaction';

export interface NodeStatusResponse {
  version: string;
  peers_connected: number;
  block_height: number;
  top_block_hash: string;
  /** Hash of the local genesis block; identifies the network. Empty (or all-zero)
   *  when the chain is empty. */
  genesis_hash: string;
}

export interface MempoolEntry {
  tx: TransactionViewResponse;
}

export interface MempoolResponse {
  count: number;
  transactions: MempoolEntry[];
}

export interface NewPeerConnectionResponse {
  success: boolean;
  fail_message: string | null;
}
