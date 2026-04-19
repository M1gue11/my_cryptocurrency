import type { CSSProperties, ReactNode } from 'react';

function joinClasses(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(' ');
}

export type ConsoleTone = 'accent' | 'good' | 'warn' | 'neutral' | 'danger';

interface ConsolePillProps {
  children: ReactNode;
  tone?: ConsoleTone;
  dot?: boolean;
  className?: string;
}

export function ConsolePill({
  children,
  tone = 'neutral',
  dot = false,
  className,
}: ConsolePillProps) {
  return (
    <span className={joinClasses('crm-pill', `crm-pill--${tone}`, className)}>
      {dot && <span className="crm-pill__dot" />}
      {children}
    </span>
  );
}

interface ConsoleButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  tone?: 'default' | 'primary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  loadingText?: string;
}

export function ConsoleButton({
  children,
  tone = 'default',
  size = 'md',
  className,
  loading = false,
  loadingText = 'Loading...',
  disabled,
  ...props
}: ConsoleButtonProps) {
  return (
    <button
      className={joinClasses(
        'crm-btn',
        `crm-btn--${tone}`,
        `crm-btn--${size}`,
        className,
      )}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? loadingText : children}
    </button>
  );
}

interface ConsolePageHeaderProps {
  eyebrow: string;
  title: string;
  actions?: ReactNode;
}

export function ConsolePageHeader({
  eyebrow,
  title,
  actions,
}: ConsolePageHeaderProps) {
  return (
    <div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
      <div>
        <div className="crm-eyebrow">{eyebrow}</div>
        <h1 className="crm-h1">{title}</h1>
      </div>
      {actions ? (
        <div className="flex flex-wrap items-center gap-2">{actions}</div>
      ) : null}
    </div>
  );
}

interface ConsolePanelProps {
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  chip?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  padded?: boolean;
  className?: string;
  bodyClassName?: string;
}

export function ConsolePanel({
  title,
  subtitle,
  icon,
  chip,
  action,
  children,
  padded = true,
  className,
  bodyClassName,
}: ConsolePanelProps) {
  return (
    <section className={joinClasses('crm-panel', className)}>
      <header className="crm-panel__header">
        <div className="flex min-w-0 items-center gap-3">
          {icon ? <span className="crm-panel__icon">{icon}</span> : null}
          <div className="min-w-0">
            <div className="crm-panel__title">{title}</div>
            {subtitle ? <div className="crm-panel__subtitle">{subtitle}</div> : null}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {chip}
          {action}
        </div>
      </header>
      <div
        className={joinClasses(
          padded ? 'crm-panel__body' : '',
          bodyClassName,
        )}
      >
        {children}
      </div>
    </section>
  );
}

interface ConsoleRowProps {
  label: string;
  value: ReactNode;
  mono?: boolean;
  className?: string;
}

export function ConsoleRow({
  label,
  value,
  mono = true,
  className,
}: ConsoleRowProps) {
  return (
    <div className={joinClasses('crm-row', mono ? 'crm-mono' : '', className)}>
      <span className="crm-row__label">{label}</span>
      <span className="crm-row__value">{value}</span>
    </div>
  );
}

interface ConsoleStatStripProps {
  children: ReactNode;
  columns?: number;
}

export function ConsoleStatStrip({
  children,
  columns = 4,
}: ConsoleStatStripProps) {
  const style = {
    '--crm-columns': String(columns),
  } as CSSProperties;

  return (
    <div className="crm-stat-strip" style={style}>
      {children}
    </div>
  );
}

interface ConsoleStatProps {
  label: string;
  value: ReactNode;
  subtitle?: ReactNode;
  tone?: ConsoleTone;
}

export function ConsoleStat({
  label,
  value,
  subtitle,
  tone = 'neutral',
}: ConsoleStatProps) {
  return (
    <div className="crm-stat">
      <div className="crm-stat__label">{label}</div>
      <div className={joinClasses('crm-stat__value', `crm-text--${tone}`)}>
        {value}
      </div>
      {subtitle ? <div className="crm-stat__subtitle">{subtitle}</div> : null}
    </div>
  );
}

interface ConsoleEmptyProps {
  title: string;
  hint?: string;
  action?: ReactNode;
}

export function ConsoleEmpty({
  title,
  hint,
  action,
}: ConsoleEmptyProps) {
  return (
    <div className="crm-empty">
      <div className="crm-empty__glyph">[]</div>
      <div className="crm-empty__title">{title}</div>
      {hint ? <div className="crm-empty__hint">{hint}</div> : null}
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}

interface ConsoleTabsProps<T extends string> {
  items: Array<{ key: T; label: string }>;
  active: T;
  onChange: (key: T) => void;
  trailing?: ReactNode;
}

export function ConsoleTabs<T extends string>({
  items,
  active,
  onChange,
  trailing,
}: ConsoleTabsProps<T>) {
  return (
    <div className="crm-tabs">
      <div className="flex flex-wrap gap-1">
        {items.map((item) => (
          <button
            key={item.key}
            className={joinClasses(
              'crm-tab',
              active === item.key && 'crm-tab--active',
            )}
            onClick={() => onChange(item.key)}
            type="button"
          >
            {item.label}
          </button>
        ))}
      </div>
      {trailing ? <div className="crm-tabs__trailing">{trailing}</div> : null}
    </div>
  );
}

export function shortHash(value: string | undefined, edge = 8) {
  if (!value) return '-';
  if (value.length <= edge * 2) return value;
  return `${value.slice(0, edge)}...${value.slice(-edge)}`;
}

export function formatCount(value: number | undefined | null) {
  return (value ?? 0).toLocaleString();
}

export function formatTimestamp(value: string | null | undefined) {
  if (!value) return '-';
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString();
}

export function formatRelativeTimestamp(value: string | number | null | undefined) {
  if (!value) return '-';
  const date = typeof value === 'number' ? new Date(value) : new Date(value);
  if (Number.isNaN(date.getTime())) return '-';
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}

export function sumTransactionOutputs<
  T extends { outputs: Array<{ value: number }> },
>(tx: T) {
  return tx.outputs.reduce((total, output) => total + output.value, 0);
}

