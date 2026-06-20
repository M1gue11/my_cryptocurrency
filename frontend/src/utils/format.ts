const COIN = 1_000_000;

export type HashPreset = 'table' | 'stat' | 'detail' | 'row';

const HASH_EDGES: Record<HashPreset, number> = {
  table: 6,
  stat: 8,
  detail: 10,
  row: 14,
};

export function compactHash(value: string | undefined | null, edge?: number): string {
  if (!value) return '-';
  const resolvedEdge = edge ?? 8;
  if (value.length <= resolvedEdge * 2 + 3) return value;
  return `${value.slice(0, resolvedEdge)}…${value.slice(-resolvedEdge)}`;
}

export function splitHash(value: string | undefined | null, edge?: number) {
  if (!value) return null;
  const resolvedEdge = edge ?? 8;
  if (value.length <= resolvedEdge * 2 + 3) {
    return { prefix: value, suffix: '', truncated: false };
  }
  return {
    prefix: value.slice(0, resolvedEdge),
    suffix: value.slice(-resolvedEdge),
    truncated: true,
  };
}

export function hashEdge(preset: HashPreset): number {
  return HASH_EDGES[preset];
}

export function formatCount(value: number | undefined | null): string {
  return (value ?? 0).toLocaleString();
}

export function formatNonce(value: number | undefined | null): string {
  if (value == null) return '-';
  return value.toLocaleString();
}

export function formatValue(
  value: number | undefined | null,
  options: { decimals?: number; suffix?: string } = {},
): string {
  const { decimals = 3, suffix = ' units' } = options;
  if (value == null) return '-';
  const coins = value / COIN;
  return `${coins.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })}${suffix}`;
}

export function formatElapsed(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainingSeconds = seconds % 60;
  return [hours, minutes, remainingSeconds]
    .map((part) => String(part).padStart(2, '0'))
    .join(':');
}

export function formatTimestamp(value: string | null | undefined): string {
  if (!value) return '-';
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
}

export function formatRelativeTimestamp(
  value: string | number | null | undefined,
): string {
  if (value == null) return '-';
  const date = typeof value === 'number' ? new Date(value) : new Date(value);
  if (Number.isNaN(date.getTime())) return '-';
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}

export function parseApiDate(raw: string): Date {
  return new Date(`${raw.slice(0, 19)}Z`);
}

export function sumTransactionOutputs<
  T extends { outputs: Array<{ value: number }> },
>(tx: T): number {
  return tx.outputs.reduce((total, output) => total + output.value, 0);
}

export function formatBlockHeight(
  height: number | undefined | null,
  options: { prefix?: boolean } = {},
): string {
  const { prefix = true } = options;
  if (height == null) return '-';
  return prefix ? `#${formatCount(height)}` : formatCount(height);
}

/** node_status.block_height is block COUNT; tip index is count - 1 */
export function tipHeightFromCount(blockCount: number | undefined | null): number | null {
  if (blockCount == null || blockCount <= 0) return null;
  return blockCount - 1;
}

/** @deprecated Use compactHash instead */
export function shortHash(value: string | undefined, edge = 8) {
  return compactHash(value, edge);
}
