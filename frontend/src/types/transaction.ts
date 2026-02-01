// Transaction Types
// Mirrors: project/src/daemon/types/tx.rs

export interface TxInputInfo {
  prev_tx_id: string;
  output_index: number;
  signature: string;
  public_key: string;
}

export interface TxOutputInfo {
  value: number;
  address: string;
}

export interface TransactionViewParams {
  id: string;
}

export interface TransactionViewResponse {
  id: string;
  date: string;
  message?: string;
  inputs: TxInputInfo[];
  outputs: TxOutputInfo[];
  is_coinbase: boolean;
  size: number;
}
