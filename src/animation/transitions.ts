// Shared Framer Motion presets. Only used for discrete UI (menus, labels,
// badges); continuous hover magnification runs in MagnifyEngine instead.

import type { Transition, Variants } from "framer-motion";

export function popTransition(speed: number): Transition {
  return { type: "spring", stiffness: 520 * speed, damping: 34, mass: 0.8 };
}

export function fadeTransition(speed: number): Transition {
  return { duration: 0.14 / Math.max(0.25, speed), ease: [0.2, 0.8, 0.2, 1] };
}

export const menuVariants: Variants = {
  hidden: { opacity: 0, scale: 0.94, y: 6 },
  shown: { opacity: 1, scale: 1, y: 0 },
};

export const labelVariants: Variants = {
  hidden: { opacity: 0, y: 4 },
  shown: { opacity: 1, y: 0 },
};

export const badgeVariants: Variants = {
  hidden: { opacity: 0, scale: 0.4 },
  shown: { opacity: 1, scale: 1 },
};
