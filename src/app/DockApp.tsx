import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getIpc, isTauri, type DockIpc } from "../lib/ipc";
import type { DockState } from "../lib/types";
import { applyTheme, surfaceMode } from "../theme/theme";
import { Dock } from "../features/dock/Dock";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function DockApp() {
  const [ipc, setIpc] = useState<DockIpc | null>(null);
  const [state, setState] = useState<DockState | null>(null);
  const readySent = useRef(false);

  useEffect(() => {
    let disposed = false;
    const unlisten: (() => void)[] = [];

    const load = async (ipc: DockIpc) => {
      for (let attempt = 0; attempt < 4; attempt++) {
        try {
          const next = await ipc.getDockState();
          if (disposed) return;
          for (const w of next.warnings) console.warn("[config]", w);
          setState(next);
          return;
        } catch (err) {
          console.warn("[dock] state not ready:", err);
          await sleep(150 * (attempt + 1));
        }
      }
    };

    void (async () => {
      const ipc = await getIpc();
      const offState = await ipc.onStateChanged(() => void load(ipc));
      const offLayout = await ipc.onLayoutChanged((layout) => setState((s) => (s ? { ...s, layout } : s)));
      if (disposed) {
        offState();
        offLayout();
        return;
      }
      unlisten.push(offState, offLayout);
      setIpc(ipc);
      await load(ipc);
    })();

    return () => {
      disposed = true;
      unlisten.forEach((off) => off());
    };
  }, []);

  useLayoutEffect(() => {
    if (state) applyTheme(state.appearance, surfaceMode(state.backdrop, isTauri));
  }, [state]);

  // Tell the native side we have painted content, so the window is shown
  // only once there is something to show. (No rAF: hidden windows may not tick.)
  useEffect(() => {
    if (state && ipc && !readySent.current) {
      readySent.current = true;
      void ipc.dockReady();
    }
  }, [state, ipc]);

  if (!state || !ipc) return null;
  return <Dock state={state} ipc={ipc} />;
}
