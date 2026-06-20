import { useReducedMotion } from 'motion/react';
import { motion } from 'motion/react';

interface MiningRingProps {
  active: boolean;
  size?: number;
}

export function MiningRing({ active, size = 96 }: MiningRingProps) {
  const shouldReduce = useReducedMotion();

  return (
    <div
      className={`crm-mining-ring ${active ? 'crm-mining-ring--active' : 'crm-mining-ring--idle'}`}
      style={{ width: size, height: size }}
    >
      <div className="crm-mining-ring__track" />

      {active && !shouldReduce ? (
        <>
          <motion.div
            className="crm-mining-ring__spin-outer"
            animate={{ rotate: 360 }}
            transition={{ duration: 1.6, repeat: Infinity, ease: 'linear' }}
          />
          <motion.div
            className="crm-mining-ring__spin-inner"
            animate={{ rotate: -360 }}
            transition={{ duration: 2.4, repeat: Infinity, ease: 'linear' }}
          />
        </>
      ) : null}

      <div className="crm-mining-ring__core">
        {active ? 'HASH' : 'IDLE'}
      </div>
    </div>
  );
}
