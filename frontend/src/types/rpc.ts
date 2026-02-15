// JSON-RPC 2.0 Protocol Types
// Mirrors: project/src/daemon/types/rpc.rs

export interface RpcRequest {
  jsonrpc: string;
  method: string;
  params: unknown;
  id?: number;
}

export interface RpcResponse<T = unknown> {
  jsonrpc: string;
  result?: T;
  error?: RpcError;
  id?: number;
}

export interface RpcError {
  code: number;
  message: string;
  data?: unknown;
}

// Standard JSON-RPC 2.0 Error Codes
export const RPC_ERROR_CODES = {
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,
} as const;
