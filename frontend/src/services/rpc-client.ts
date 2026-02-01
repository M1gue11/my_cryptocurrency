// RPC Client for communicating with the Caramuru daemon
// Uses HTTP (requires HTTP endpoint in daemon or proxy)

import type {
  RpcRequest,
  RpcResponse,
  NodeStatusResponse,
  MempoolResponse,
  ChainStatusResponse,
  ChainShowResponse,
  UtxosResponse,
  MineBlockResponse,
  WalletNewResponse,
  WalletAddressResponse,
  WalletBalanceResponse,
  WalletSendResponse,
  WalletGenerateKeysResponse,
  TransactionViewResponse,
  WalletAccessParams,
} from '../types';

// Global request ID counter
let requestId = 1;

export class RpcClient {
  private baseUrl: string;

  constructor(baseUrl: string = 'http://localhost:7000') {
    this.baseUrl = baseUrl;
  }

  /**
   * Makes a generic JSON-RPC call
   */
  async call<T>(method: string, params: unknown = {}): Promise<T> {
    const request: RpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id: requestId++,
    };

    const response = await fetch(this.baseUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`HTTP error: ${response.status} ${response.statusText}`);
    }

    const rpcResponse: RpcResponse<T> = await response.json();

    if (rpcResponse.error) {
      throw new Error(`RPC error: ${rpcResponse.error.message} (code: ${rpcResponse.error.code})`);
    }

    if (rpcResponse.result === undefined) {
      throw new Error('Empty response from daemon');
    }

    return rpcResponse.result;
  }

  /**
   * Check if daemon is reachable
   */
  async ping(): Promise<boolean> {
    try {
      await this.nodeStatus();
      return true;
    } catch {
      return false;
    }
  }

  // ==========================================================================
  // Node Methods
  // ==========================================================================

  async nodeStatus(): Promise<NodeStatusResponse> {
    return this.call<NodeStatusResponse>('node_status');
  }

  async nodeInit(): Promise<{ success: boolean; block_count: number }> {
    return this.call('node_init');
  }

  async nodeMempool(): Promise<MempoolResponse> {
    return this.call<MempoolResponse>('node_mempool');
  }

  async nodeClearMempool(): Promise<{ success: boolean }> {
    return this.call('node_clear_mempool');
  }

  async nodeSave(): Promise<{ success: boolean }> {
    return this.call('node_save');
  }

  // ==========================================================================
  // Mining Methods
  // ==========================================================================

  async mineBlock(): Promise<MineBlockResponse> {
    return this.call<MineBlockResponse>('mine_block');
  }

  // ==========================================================================
  // Chain Methods
  // ==========================================================================

  async chainStatus(): Promise<ChainStatusResponse> {
    return this.call<ChainStatusResponse>('chain_status');
  }

  async chainShow(): Promise<ChainShowResponse> {
    return this.call<ChainShowResponse>('chain_show');
  }

  async chainValidate(): Promise<{ valid: boolean; error?: string }> {
    return this.call('chain_validate');
  }

  async chainUtxos(limit: number = 20): Promise<UtxosResponse> {
    return this.call<UtxosResponse>('chain_utxos', { limit });
  }

  // ==========================================================================
  // Wallet Methods
  // ==========================================================================

  async walletNew(password: string, path: string): Promise<WalletNewResponse> {
    return this.call<WalletNewResponse>('wallet_new', { password, path });
  }

  async walletAddress(keyPath: string, password: string): Promise<WalletAddressResponse> {
    return this.call<WalletAddressResponse>('wallet_address', {
      key_path: keyPath,
      password,
    });
  }

  async walletBalance(keyPath: string, password: string): Promise<WalletBalanceResponse> {
    return this.call<WalletBalanceResponse>('wallet_balance', {
      key_path: keyPath,
      password,
    });
  }

  async walletSend(
    from: WalletAccessParams,
    to: string,
    amount: number,
    fee?: number,
    message?: string
  ): Promise<WalletSendResponse> {
    return this.call<WalletSendResponse>('wallet_send', {
      from,
      to,
      amount,
      fee,
      message,
    });
  }

  async walletGenerateKeys(
    wallet: WalletAccessParams,
    count: number = 5,
    derivationType?: number
  ): Promise<WalletGenerateKeysResponse> {
    return this.call<WalletGenerateKeysResponse>('wallet_generate_keys', {
      wallet,
      count,
      derivation_type: derivationType,
    });
  }

  // ==========================================================================
  // Transaction Methods
  // ==========================================================================

  async transactionView(id: string): Promise<TransactionViewResponse> {
    return this.call<TransactionViewResponse>('transaction_view', { id });
  }
}

// Default client instance
export const rpcClient = new RpcClient();
