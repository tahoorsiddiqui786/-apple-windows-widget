import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./theme/tokens.css";
import { DockApp } from "./app/DockApp";

// A dock is not a web page: switch off browser behaviours that would leak
// through (default context menu, drag-out of icons, zoom, reload, find, print).
window.addEventListener("contextmenu", (e) => e.preventDefault());
window.addEventListener("dragstart", (e) => e.preventDefault());
window.addEventListener(
  "wheel",
  (e) => {
    if (e.ctrlKey) e.preventDefault();
  },
  { passive: false },
);
window.addEventListener("keydown", (e) => {
  const k = e.key.toLowerCase();
  if (k === "f5" || k === "f3" || (e.ctrlKey && ["r", "f", "g", "p", "u", "s", "+", "-", "="].includes(k))) {
    e.preventDefault();
  }
});

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <DockApp />
    </StrictMode>,
  );
}
