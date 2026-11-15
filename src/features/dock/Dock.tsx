import { useCallback, useEffect, useLayoutEffect, useRef, useState, type MouseEvent } from "react";
import { AnimatePresence, motion } from "framer-motion";
import type { DockIpc } from "../../lib/ipc";
import type { DockState, WindowEntry } from "../../lib/types";
import { fadeTransition, labelVariants } from "../../animation/transitions";
import { MagnifyEngine } from "./magnifyEngine";
import { DockItem } from "./DockItem";
import { DockContextMenu, type MenuAnchor } from "./DockContextMenu";
import "./dock.css";

interface Props {
  state: DockState;
  ipc: DockIpc;
}

const BOUNCE_MS = 950;
const ERROR_MS = 2800;
const MENU_LEAVE_GRACE_MS = 700;

function errorText(err: unknown): string {
  if (err instanceof Error) return err.message;
  return typeof err === "string" ? err : "Unknown error";
}

export function Dock({ state, ipc }: Props) {
  const { layout, dock, appearance, items, hotkey, running } = state;
  const speed = Math.max(0.25, appearance.animationSpeed);
  // Read inside stable callbacks without re-creating them (keeps DockItem memoized).
  const runningRef = useRef(running);
  runningRef.current = running;

  const barRef = useRef<HTMLDivElement>(null);
  const glowRef = useRef<HTMLDivElement>(null);
  const labelRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);
  const engineRef = useRef<MagnifyEngine | null>(null);
  const pointer = useRef({ x: -1, y: -1 });
  const labelIndexRef = useRef(-1);
  const bouncingRef = useRef(new Set<string>());
  const menuRelease = useRef<(() => void) | null>(null);
  const timers = useRef(new Set<number>());

  const [hoverIndex, setHoverIndex] = useState(-1);
  const [menu, setMenu] = useState<MenuAnchor | null>(null);
  const [bouncing, setBouncing] = useState<ReadonlySet<string>>(() => new Set());
  const [shaking, setShaking] = useState<string | null>(null);
  const [error, setError] = useState<{ index: number; message: string } | null>(null);
  const [menuWindows, setMenuWindows] = useState<WindowEntry[] | null>(null);
  const menuRequest = useRef(0);

  const later = useCallback((fn: () => void, ms: number) => {
    const id = window.setTimeout(() => {
      timers.current.delete(id);
      fn();
    }, ms);
    timers.current.add(id);
    return id;
  }, []);

  useEffect(() => {
    const pending = timers.current;
    return () => pending.forEach((id) => window.clearTimeout(id));
  }, []);

  const setItemRef = useCallback((i: number, el: HTMLDivElement | null) => {
    itemRefs.current[i] = el;
  }, []);

  const labelIndex = menu ? -1 : error ? error.index : dock.showLabels ? hoverIndex : -1;
  labelIndexRef.current = labelIndex;

  // ---------------------------------------------------------------- engine
  useLayoutEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    itemRefs.current.length = items.length;
    const engine = new MagnifyEngine(
      {
        count: items.length,
        iconSize: dock.iconSize,
        spacing: dock.iconSpacing,
        padding: dock.padding,
        maxScale: dock.magnification,
        range: dock.magnificationRange,
      },
      { barLeft: layout.barLeft, barTop: layout.barTop, barWidth: layout.barWidth, barHeight: layout.barHeight },
      {
        bar,
        items: itemRefs.current,
        glow: glowRef.current,
        label: () => ({ el: labelRef.current, index: labelIndexRef.current }),
      },
      {
        onInteractive: (expanded) => void ipc.setDockInteractive(expanded),
        onExtent: (l, r) => ipc.setBackdropExtent(l, r),
        onHoverIndex: setHoverIndex,
      },
      speed,
    );
    engineRef.current = engine;
    return () => {
      engine.destroy();
      engineRef.current = null;
      void ipc.setDockInteractive(false);
      void ipc.setBackdropExtent(0, 0);
    };
  }, [ipc, items.length, layout, dock, speed]);

  // Keep the label glued to its icon even when the engine is idle.
  useLayoutEffect(() => {
    engineRef.current?.positionLabel();
  }, [labelIndex]);

  // ---------------------------------------------------------------- pointer
  const closeMenu = useCallback(() => {
    setMenu(null);
    const p = pointer.current;
    engineRef.current?.setFrozen(false, p.x >= 0 ? p.x : undefined, p.y >= 0 ? p.y : undefined);
    menuRelease.current?.();
    menuRelease.current = null;
  }, []);

  const leaveTimer = useRef(0);
  useEffect(() => {
    const move = (e: PointerEvent) => {
      pointer.current = { x: e.clientX, y: e.clientY };
      if (leaveTimer.current) {
        window.clearTimeout(leaveTimer.current);
        leaveTimer.current = 0;
      }
      engineRef.current?.pointerMove(e.clientX, e.clientY);
    };
    const leave = () => {
      pointer.current = { x: -1, y: -1 };
      engineRef.current?.leave();
    };
    window.addEventListener("pointermove", move, { passive: true });
    document.documentElement.addEventListener("pointerleave", leave);
    return () => {
      window.removeEventListener("pointermove", move);
      document.documentElement.removeEventListener("pointerleave", leave);
    };
  }, []);

  // While the menu is open: close on any outside press, or shortly after the
  // pointer leaves the dock (clicks in other apps never reach this window).
  useEffect(() => {
    if (!menu) return;
    const onDown = () => closeMenu();
    const onLeave = () => {
      leaveTimer.current = window.setTimeout(() => {
        leaveTimer.current = 0;
        closeMenu();
      }, MENU_LEAVE_GRACE_MS);
    };
    window.addEventListener("pointerdown", onDown);
    document.documentElement.addEventListener("pointerleave", onLeave);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      document.documentElement.removeEventListener("pointerleave", onLeave);
      if (leaveTimer.current) window.clearTimeout(leaveTimer.current);
      leaveTimer.current = 0;
    };
  }, [menu, closeMenu]);

  // ---------------------------------------------------------------- actions
  const showError = useCallback(
    (index: number, id: string, message: string) => {
      const release = engineRef.current?.hold();
      setShaking(id);
      setError({ index, message });
      later(() => setShaking((s) => (s === id ? null : s)), 450);
      later(() => {
        setError((e) => (e?.index === index ? null : e));
        release?.();
      }, ERROR_MS);
    },
    [later],
  );

  /** Bounces the icon while `task` (a launch) runs; shakes and explains on failure. */
  const bounceWhile = useCallback(
    (id: string, task: () => Promise<unknown>) => {
      if (bouncingRef.current.has(id)) return;
      const index = items.findIndex((i) => i.id === id);
      const label = items[index]?.label ?? id;
      bouncingRef.current.add(id);
      setError(null);
      setBouncing(new Set(bouncingRef.current));
      const release = engineRef.current?.hold();
      let finished = false;
      const finish = () => {
        if (finished) return;
        finished = true;
        bouncingRef.current.delete(id);
        setBouncing(new Set(bouncingRef.current));
        release?.();
      };
      const timer = later(finish, BOUNCE_MS / speed);
      task().catch((err: unknown) => {
        window.clearTimeout(timer);
        timers.current.delete(timer);
        finish();
        showError(index, id, `Couldn't open ${label}. ${errorText(err)}`);
      });
    },
    [items, later, showError, speed],
  );

  /** Always opens a new instance/window (Shift+click, middle-click, menu). */
  const launch = useCallback((id: string) => bounceWhile(id, () => ipc.launchItem(id)), [bounceWhile, ipc]);

  /** Plain click: switch to the app if it is open, otherwise launch it. */
  const open = useCallback(
    (id: string, newWindow: boolean) => {
      if (newWindow) return launch(id);
      if (!runningRef.current[id]) return bounceWhile(id, () => ipc.activateItem(id));
      ipc.activateItem(id).catch((err: unknown) => {
        const index = items.findIndex((i) => i.id === id);
        showError(index, id, errorText(err));
      });
    },
    [bounceWhile, ipc, items, launch, showError],
  );

  const openMenu = useCallback(
    (e: MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      const engine = engineRef.current;
      const target = e.target as HTMLElement;
      if (!engine || !target.closest(".dock-bar, .dock-item")) return;
      const itemEl = target.closest<HTMLElement>("[data-item-id]");
      const index = itemEl ? items.findIndex((i) => i.id === itemEl.dataset.itemId) : -1;
      engine.setFrozen(true);
      menuRelease.current?.();
      menuRelease.current = engine.hold();
      const box = engine.iconBox(index);
      const item = index >= 0 ? (items[index] ?? null) : null;
      setError(null);
      setMenu({
        item,
        x: box ? box.cx : e.clientX,
        bottom: (box ? box.top : layout.barTop) - 10,
      });
      // Window titles are read only now, for this menu, and never kept.
      const request = ++menuRequest.current;
      setMenuWindows(null);
      if (item && runningRef.current[item.id]) {
        ipc
          .listItemWindows(item.id)
          .catch(() => [])
          .then((list) => {
            if (menuRequest.current === request) setMenuWindows(list);
          });
      }
    },
    [ipc, items, layout.barTop],
  );

  const hideDock = useCallback(() => {
    engineRef.current?.reset();
    void ipc.hideDock();
  }, [ipc]);

  const reportFailure = useCallback(
    (id: string) => (err: unknown) => {
      const index = items.findIndex((i) => i.id === id);
      showError(index, id, errorText(err));
    },
    [items, showError],
  );

  // ---------------------------------------------------------------- render
  const labelItem = labelIndex >= 0 ? items[labelIndex] : undefined;
  return (
    <div className="dock-root" onContextMenu={openMenu}>
      <div ref={barRef} className="dock-bar" style={{ height: layout.barHeight }}>
        <div ref={glowRef} className="dock-glow" />
      </div>

      {items.map((item, i) => (
        <DockItem
          key={item.id}
          item={item}
          index={i}
          running={running[item.id] ?? 0}
          bouncing={bouncing.has(item.id)}
          shaking={shaking === item.id}
          speed={speed}
          setRef={setItemRef}
          onOpen={open}
        />
      ))}

      <div ref={labelRef} className="dock-label-anchor">
        <div className="dock-label-center">
          <AnimatePresence>
            {labelItem && (
              <motion.div
                key="label"
                className={error ? "dock-label is-error" : "dock-label"}
                variants={labelVariants}
                initial="hidden"
                animate="shown"
                exit="hidden"
                transition={fadeTransition(speed)}
              >
                {error ? error.message : labelItem.label}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

      <AnimatePresence>
        {menu && (
          <DockContextMenu
            key="menu"
            anchor={menu}
            viewportWidth={layout.width}
            hotkey={hotkey}
            speed={speed}
            windows={menuWindows}
            running={menu.item ? (running[menu.item.id] ?? 0) : 0}
            onOpen={(id) => open(id, false)}
            onNewWindow={launch}
            onFocusWindow={(id, handle) => void ipc.focusItemWindow(id, handle).catch(reportFailure(id))}
            onReveal={(id) => void ipc.revealItem(id).catch(reportFailure(id))}
            onHide={hideDock}
            onEditConfig={() => void ipc.openConfig()}
            onClose={closeMenu}
          />
        )}
      </AnimatePresence>
    </div>
  );
}
