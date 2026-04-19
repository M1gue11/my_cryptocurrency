import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import {
  ConsoleButton,
  ConsoleEmpty,
  ConsolePageHeader,
  ConsolePanel,
  ConsolePill,
} from '../components';
import { rpcClient } from '../services';
import type { LogEntry, LogCategory, LogLevel } from '../types';

const CATEGORIES: Array<'all' | LogCategory> = ['all', 'Core', 'P2P', 'RPC'];
const LEVELS: Array<'all' | LogLevel> = ['all', 'Info', 'Warning', 'Error'];

function normalizeCategory(value: 'all' | LogCategory) {
  return value === 'all' ? undefined : value.toLowerCase();
}

function normalizeLevel(value: 'all' | LogLevel) {
  if (value === 'all') return undefined;
  if (value === 'Warning') return 'warn';
  if (value === 'Info') return 'info';
  return 'error';
}

function formatLogTimestamp(raw: string) {
  const parsed = new Date(raw);
  if (Number.isNaN(parsed.getTime())) return raw;
  return parsed.toLocaleTimeString();
}

export function Logs() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<'all' | LogCategory>('all');
  const [selectedLevel, setSelectedLevel] = useState<'all' | LogLevel>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [limit, setLimit] = useState(100);

  const loadLogs = useCallback(async (background = false) => {
    try {
      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }
      setError(null);

      const response = await rpcClient.getLogs(
        normalizeCategory(selectedCategory),
        normalizeLevel(selectedLevel),
        limit,
      );

      setLogs(response);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : 'Failed to fetch logs');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [limit, selectedCategory, selectedLevel]);

  useEffect(() => {
    void loadLogs();
  }, [limit, loadLogs, selectedCategory, selectedLevel]);

  const filteredLogs = useMemo(() => {
    if (!searchQuery.trim()) return logs;
    const query = searchQuery.toLowerCase();
    return logs.filter(
      (entry) =>
        entry.message.toLowerCase().includes(query) ||
        entry.category.toLowerCase().includes(query) ||
        entry.level.toLowerCase().includes(query),
    );
  }, [logs, searchQuery]);

  const levelCounts = useMemo(() => {
    const counts = { Info: 0, Warning: 0, Error: 0 };
    for (const entry of logs) {
      counts[entry.level] += 1;
    }
    return counts;
  }, [logs]);

  if (loading && logs.length === 0) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center">
        <div className="crm-mono text-sm text-[var(--crm-dim)]">
          Loading diagnostics...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <ConsolePageHeader
        eyebrow="get_logs"
        title="Logs & Diagnostics"
        actions={
          <>
            <ConsolePill>{filteredLogs.length} events</ConsolePill>
            <ConsoleButton onClick={() => void loadLogs(true)} loading={refreshing}>
              refresh
            </ConsoleButton>
          </>
        }
      />

      {error ? (
        <div className="rounded-sm border border-[var(--crm-warn)]/40 bg-[var(--crm-warn-bg)] px-4 py-3 text-sm text-[var(--crm-warn)]">
          {error}
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-3">
        <SummaryCard label="Info" count={levelCounts.Info} tone="accent" />
        <SummaryCard label="Warning" count={levelCounts.Warning} tone="warn" />
        <SummaryCard label="Error" count={levelCounts.Error} tone="danger" />
      </div>

      <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-4">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-center">
          <FilterGroup
            label="level"
            value={selectedLevel}
            onChange={setSelectedLevel}
            options={LEVELS}
          />
          <FilterGroup
            label="category"
            value={selectedCategory}
            onChange={setSelectedCategory}
            options={CATEGORIES}
          />
          <div className="flex items-center gap-2">
            <div className="crm-field-label mb-0">limit</div>
            <select
              className="crm-select w-[92px]"
              value={limit}
              onChange={(event) => setLimit(Number(event.target.value))}
            >
              {[50, 100, 200, 500].map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </div>
          <div className="min-w-0 flex-1">
            <input
              className="crm-input"
              placeholder="Search message text, category, or level"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
            />
          </div>
        </div>
      </div>

      <ConsolePanel
        title="event log"
        subtitle={`${filteredLogs.length} / ${logs.length}`}
        icon="="
        padded={false}
      >
        {filteredLogs.length > 0 ? (
          <div className="crm-mono text-xs">
            {filteredLogs.map((entry, index) => (
              <div
                key={`${entry.timestamp}-${entry.category}-${index}`}
                className="grid gap-3 border-t border-[var(--crm-border)] px-4 py-3 first:border-t-0 md:grid-cols-[92px_72px_72px_minmax(0,1fr)] md:items-center"
              >
                <span className="text-[var(--crm-dim)]">
                  {formatLogTimestamp(entry.timestamp)}
                </span>
                <span>
                  <ConsolePill
                    tone={
                      entry.level === 'Error'
                        ? 'danger'
                        : entry.level === 'Warning'
                          ? 'warn'
                          : 'neutral'
                    }
                  >
                    {entry.level === 'Warning' ? 'WARN' : entry.level.toUpperCase()}
                  </ConsolePill>
                </span>
                <span className="text-[var(--crm-muted)]">[{entry.category}]</span>
                <span className="whitespace-pre-wrap text-[var(--crm-fg)]">
                  {entry.message}
                </span>
              </div>
            ))}
          </div>
        ) : (
          <ConsoleEmpty
            title="no events match the current filter"
            hint="Try widening the selected level or category, or clear the search query."
          />
        )}
      </ConsolePanel>
    </div>
  );
}

function SummaryCard({
  label,
  count,
  tone,
}: {
  label: string;
  count: number;
  tone: 'accent' | 'warn' | 'danger';
}) {
  return (
    <div className="rounded-sm border border-[var(--crm-border)] bg-[var(--crm-panel)] p-4">
      <div className="crm-field-label">{label}</div>
      <div
        className={`crm-mono text-3xl tracking-[-0.04em] ${
          tone === 'accent'
            ? 'text-[var(--crm-accent)]'
            : tone === 'warn'
              ? 'text-[var(--crm-warn)]'
              : 'text-[var(--crm-bad)]'
        }`}
      >
        {count}
      </div>
    </div>
  );
}

function FilterGroup<T extends string>({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: readonly T[];
}) {
  return (
    <div className="flex items-center gap-2">
      <div className="crm-field-label mb-0">{label}</div>
      <div className="flex flex-wrap gap-1 rounded-sm border border-[var(--crm-border)] bg-[var(--crm-bg-2)] p-1">
        {options.map((option) => {
          const active = option === value;
          return (
            <button
              key={option}
              className={`rounded-sm px-3 py-1 crm-mono text-[10px] uppercase tracking-[0.08em] transition ${
                active
                  ? 'bg-[var(--crm-accent-bg)] text-[var(--crm-accent)]'
                  : 'text-[var(--crm-muted)]'
              }`}
              onClick={() => onChange(option)}
              type="button"
            >
              {option.toLowerCase()}
            </button>
          );
        })}
      </div>
    </div>
  );
}
