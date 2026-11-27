import { useEffect } from "react";

import type { Backdrop } from "./types";

/** Marks the page so the stylesheet draws opaque tints where Mica is missing. */
export function useBackdrop(backdrop: Backdrop | undefined): void {
  useEffect(() => {
    if (backdrop) {
      document.documentElement.dataset.backdrop = backdrop;
    }
  }, [backdrop]);
}
