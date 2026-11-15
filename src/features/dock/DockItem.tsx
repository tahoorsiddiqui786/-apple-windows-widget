import { memo, useCallback, type MouseEvent } from "react";
import { AnimatePresence, motion } from "framer-motion";
import type { DockItemView } from "../../lib/types";
import { badgeVariants, fadeTransition, popTransition } from "../../animation/transitions";

interface Props {
  item: DockItemView;
  index: number;
  /** Number of open windows (0 = not running). */
  running: number;
  bouncing: boolean;
  shaking: boolean;
  speed: number;
  setRef(index: number, el: HTMLDivElement | null): void;
  /** `newWindow`: Shift+click or middle-click always opens a new window. */
  onOpen(id: string, newWindow: boolean): void;
}

export const DockItem = memo(function DockItem(props: Props) {
  const { item, index, running, bouncing, shaking, speed, setRef, onOpen } = props;
  const ref = useCallback((el: HTMLDivElement | null) => setRef(index, el), [index, setRef]);
  const body = ["dock-item__body", bouncing && "is-bouncing", shaking && "is-shaking"].filter(Boolean).join(" ");
  const onClick = (e: MouseEvent) => onOpen(item.id, e.shiftKey);
  const onAuxClick = (e: MouseEvent) => {
    if (e.button === 1) onOpen(item.id, true);
  };
  return (
    <div
      ref={ref}
      className="dock-item"
      data-item-id={item.id}
      role="button"
      aria-label={running > 0 ? `${item.label} (open)` : item.label}
      onClick={onClick}
      onAuxClick={onAuxClick}
      onMouseDown={(e) => e.button === 1 && e.preventDefault() /* no autoscroll cursor */}
    >
      <div className={body}>
        {item.icon ? <img src={item.icon} alt="" draggable={false} decoding="async" /> : <Monogram label={item.label} />}
      </div>
      <AnimatePresence>
        {running > 0 && (
          <motion.span
            key="dot"
            className="dock-dot"
            initial={{ opacity: 0, scale: 0.3 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.3 }}
            transition={fadeTransition(speed)}
          />
        )}
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
