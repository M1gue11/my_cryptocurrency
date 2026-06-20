import type { ReactNode } from 'react';
import {
  compactHash,
  hashEdge,
  splitHash,
  type HashPreset,
} from '../../utils/format';
import { CopyableValue } from './CopyableValue';

function joinClasses(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(' ');
}

export type HashDisplaySize = 'xs' | 'sm' | 'md' | 'lg';

interface HashDisplayProps {
  value?: string | null;
  preset?: HashPreset;
  edge?: number;
  size?: HashDisplaySize;
  copyable?: boolean;
  className?: string;
  fallback?: ReactNode;
}

export function HashDisplay({
  value,
  preset = 'stat',
  edge,
  size = 'md',
  copyable = true,
  className,
  fallback = '-',
}: HashDisplayProps) {
  if (!value) {
    return <span className={joinClasses('crm-hash', `crm-hash--${size}`, className)}>{fallback}</span>;
  }

  const resolvedEdge = edge ?? hashEdge(preset);
  const parts = splitHash(value, resolvedEdge);
  const display = compactHash(value, resolvedEdge);

  const content = parts?.truncated ? (
    <>
      <span className="crm-hash__prefix">{parts.prefix}</span>
      <span className="crm-hash__sep">…</span>
      <span className="crm-hash__suffix">{parts.suffix}</span>
    </>
  ) : (
    <span className="crm-hash__full">{display}</span>
  );

  const classes = joinClasses('crm-hash', `crm-hash--${size}`, className);

  if (!copyable) {
    return (
      <span className={classes} title={value}>
        {content}
      </span>
    );
  }

  return (
    <CopyableValue value={value} className={classes} label="Copy hash">
      {content}
    </CopyableValue>
  );
}
