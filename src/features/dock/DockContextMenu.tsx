import { motion } from "framer-motion";
import type { DockItemView, WindowEntry } from "../../lib/types";
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
  /** Open windows of the item: null while loading, [] when none. */
  windows: WindowEntry[] | null;
  running: number;
  onOpen(id: string): void;
  onNewWindow(id: string): void;
  onFocusWindow(id: string, handle: string): void;
  onReveal(id: string): void;
  onHide(): void;
  onEditConfig(): void;
  onClose(): void;
}

const MENU_WIDTH = 248;
const MAX_LISTED = 8;

export function DockContextMenu(props: Props) {
  const { anchor, viewportWidth, hotkey, speed, windows, running } = props;
  const left = Math.min(Math.max(8, anchor.x - MENU_WIDTH / 2), viewportWidth - MENU_WIDTH - 8);
  const run = (fn: () => void) => () => {
    props.onClose();
    fn();
  };
  const item = anchor.item;
  const listed = windows?.slice(0, MAX_LISTED) ?? [];
  const hidden = (windows?.length ?? 0) - listed.length;

  return (
    // The anchor pins the menu's bottom edge above the icon; the inner element
    // is free to animate its own transform.
    <div className="dock-menu-anchor" style={{ left, top: anchor.bottom, width: MENU_WIDTH }}>
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
            {running > 0 && (
              <div className="dock-menu__windows" role="group" aria-label="Open windows">
                {windows === null && <div className="dock-menu__loading">Finding windows…</div>}
                {listed.map((w) => (
                  <button
                    key={w.handle}
                    className="dock-menu__item dock-menu__window"
                    role="menuitem"
                    title={w.title}
                    onClick={run(() => props.onFocusWindow(item.id, w.handle))}
                  >
                    <span className="dock-menu__window-title">{w.title || "Untitled window"}</span>
                    {w.minimized && <span className="dock-menu__hint">Minimized</span>}
                  </button>
                ))}
                {hidden > 0 && <div className="dock-menu__loading">+{hidden} more</div>}
              </div>
            )}
            {running > 0 && <div className="dock-menu__sep" role="separator" />}
            {running > 0 ? (
              <button className="dock-menu__item" role="menuitem" onClick={run(() => props.onNewWindow(item.id))}>
                New Window
                <span className="dock-menu__hint">Shift+Click</span>
              </button>
            ) : (
              <button className="dock-menu__item" role="menuitem" onClick={run(() => props.onOpen(item.id))}>
                Open
              </button>
            )}
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
