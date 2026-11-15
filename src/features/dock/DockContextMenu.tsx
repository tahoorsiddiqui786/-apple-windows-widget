import { motion } from "framer-motion";
import type { DockItemView } from "../../lib/types";
import { menuVariants, popTransition } from "../../animation/transitions";

export interface MenuAnchor {
  item: DockItemView | null;
  /** Window x of the menu's centre. */
  x: number;
  /** Window y the menu's bottom edge sits at. */
  bottom: number;
}

interface Props {
  anchor: MenuAnchor;
  viewportWidth: number;
  hotkey: string;
  speed: number;
  onOpen(id: string): void;
  onReveal(id: string): void;
  onHide(): void;
  onEditConfig(): void;
  onClose(): void;
}

const MENU_WIDTH = 216;

export function DockContextMenu(props: Props) {
  const { anchor, viewportWidth, hotkey, speed } = props;
  const left = Math.min(Math.max(8, anchor.x - MENU_WIDTH / 2), viewportWidth - MENU_WIDTH - 8);
  const run = (fn: () => void) => () => {
    props.onClose();
    fn();
  };
  const item = anchor.item;

  return (
    // The anchor pins the menu's bottom edge above the icon; the inner element
    // is free to animate its own transform.
    <div className="dock-menu-anchor" style={{ left, top: anchor.bottom }}>
    <motion.div
      className="dock-menu"
      role="menu"
      variants={menuVariants}
      initial="hidden"
      animate="shown"
      exit="hidden"
      transition={popTransition(speed)}
      onPointerDown={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {item && (
        <>
          <div className="dock-menu__title">{item.label}</div>
          <button className="dock-menu__item" role="menuitem" onClick={run(() => props.onOpen(item.id))}>
            Open
          </button>
          {item.canReveal && (
            <button className="dock-menu__item" role="menuitem" onClick={run(() => props.onReveal(item.id))}>
              Show in File Explorer
            </button>
          )}
          <div className="dock-menu__sep" role="separator" />
        </>
      )}
      <button className="dock-menu__item" role="menuitem" onClick={run(props.onHide)}>
        Hide Dock
        {hotkey && <span className="dock-menu__hint">{hotkey}</span>}
      </button>
      <button className="dock-menu__item" role="menuitem" onClick={run(props.onEditConfig)}>
        Edit Dock Settings…
      </button>
    </motion.div>
    </div>
  );
}
