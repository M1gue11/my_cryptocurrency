import { useReducedMotion } from 'motion/react';
import { motion } from 'motion/react';
import { formatElapsed } from '../../utils/format';

function joinClasses(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(' ');
}

interface ElapsedTimerProps {
  seconds: number;
  active?: boolean;
  size?: 'md' | 'lg' | 'xl';
  className?: string;
  placeholder?: string;
}

export function ElapsedTimer({
  seconds,
  active = false,
  size = 'lg',
  className,
  placeholder = '--:--:--',
}: ElapsedTimerProps) {
  const shouldReduce = useReducedMotion();
  const display = active ? formatElapsed(seconds) : placeholder;
  const tone = active ? 'active' : 'idle';

  if (shouldReduce) {
    return (
      <span
        className={joinClasses(
          'crm-timer',
          `crm-timer--${size}`,
          `crm-timer--${tone}`,
          className,
        )}
      >
        {display}
      </span>
    );
  }

  return (
    <motion.span
      className={joinClasses(
        'crm-timer',
        `crm-timer--${size}`,
        `crm-timer--${tone}`,
        className,
      )}
      key={display}
      initial={{ opacity: 0.6, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ type: 'spring', stiffness: 320, damping: 28 }}
    >
      {display}
    </motion.span>
  );
}
