import { useReducedMotion } from 'motion/react';
import { motion, AnimatePresence } from 'motion/react';
import { formatCount } from '../../utils/format';

function joinClasses(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(' ');
}

interface AnimatedNumberProps {
  value: number | undefined | null;
  format?: (value: number) => string;
  className?: string;
  fallback?: string;
}

export function AnimatedNumber({
  value,
  format = formatCount,
  className,
  fallback = '-',
}: AnimatedNumberProps) {
  const shouldReduce = useReducedMotion();

  if (value == null) {
    return <span className={joinClasses('crm-number', className)}>{fallback}</span>;
  }

  const formatted = format(value);

  if (shouldReduce) {
    return <span className={joinClasses('crm-number', className)}>{formatted}</span>;
  }

  return (
    <span className={joinClasses('crm-number', className)}>
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.span
          key={formatted}
          className="crm-number__value"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -6 }}
          transition={{ type: 'spring', stiffness: 380, damping: 30, duration: 0.25 }}
        >
          {formatted}
        </motion.span>
      </AnimatePresence>
    </span>
  );
}
