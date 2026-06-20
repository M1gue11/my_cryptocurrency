import { useCallback, useState } from 'react';
import { motion, AnimatePresence } from 'motion/react';

function joinClasses(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(' ');
}

interface CopyableValueProps {
  value: string;
  children: React.ReactNode;
  className?: string;
  label?: string;
}

export function CopyableValue({
  value,
  children,
  className,
  label = 'Copy',
}: CopyableValueProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      // Clipboard may be unavailable in some contexts.
    }
  }, [value]);

  return (
    <button
      type="button"
      className={joinClasses('crm-copyable', className)}
      onClick={() => void handleCopy()}
      title={`${label}: ${value}`}
      aria-label={`${label}: ${value}`}
    >
      {children}
      <AnimatePresence>
        {copied ? (
          <motion.span
            key="copied"
            className="crm-copyable__badge"
            initial={{ opacity: 0, scale: 0.8, y: 4 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.9, y: -2 }}
            transition={{ type: 'spring', stiffness: 420, damping: 28 }}
          >
            copied
          </motion.span>
        ) : null}
      </AnimatePresence>
    </button>
  );
}
