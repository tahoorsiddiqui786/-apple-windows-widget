import { memo, useCallback } from "react";
import { AnimatePresence, motion } from "framer-motion";
import type { DockItemView } from "../../lib/types";
import { badgeVariants, popTransition } from "../../animation/transitions";

interface Props {
  item: DockItemView;
  index: number;
  bouncing: boolean;
  shaking: boolean;
  speed: number;
  setRef(index: number, el: HTMLDivElement | null): void;
  onLaunch(id: string): void;
}

export const DockItem = memo(function DockItem({ item, index, bouncing, shaking, speed, setRef, onLaunch }: Props) {
  const ref = useCallback((el: HTMLDivElement | null) => setRef(index, el), [index, setRef]);
  const body = ["dock-item__body", bouncing && "is-bouncing", shaking && "is-shaking"].filter(Boolean).join(" ");
  return (
    <div
      ref={ref}
      className="dock-item"
      data-item-id={item.id}
      role="button"
      aria-label={item.label}
      onClick={() => onLaunch(item.id)}
    >
      <div className={body}>
        {item.icon ? <img src={item.icon} alt="" draggable={false} decoding="async" /> : <Monogram label={item.label} />}
      </div>
      <AnimatePresence>
        {item.badge && (
          <motion.span
            key="badge"
            className="dock-badge"
            variants={badgeVariants}
            initial="hidden"
            animate="shown"
            exit="hidden"
            transition={popTransition(speed)}
          >
            {item.badge}
          </motion.span>
        )}
      </AnimatePresence>
    </div>
  );
});

// Deterministic, on-palette tile for items without a loadable icon.
const HUES = [232, 262, 192, 212, 284, 176];

function Monogram({ label }: { label: string }) {
  let hash = 0;
  for (const ch of label) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  const hue = HUES[hash % HUES.length] ?? 232;
  const letter = [...label.trim()][0]?.toUpperCase() ?? "?";
  return (
    <div
      className="dock-monogram"
      style={{
        background: `linear-gradient(145deg, hsl(${hue} 80% 66%), hsl(${hue + 18} 70% 42%))`,
      }}
    >
      <span>{letter}</span>
    </div>
  );
}
