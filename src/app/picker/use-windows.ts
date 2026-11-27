import { useEffect, useState } from "react";

import { bridge } from "../bridge";
import type { WindowView } from "../types";

/** Windows open, close and start playing sound while the picker is shown. */
const REFRESH_MS = 2000;

/** The capturable windows, refreshed while the picker is mounted. */
export function useWindows(): WindowView[] | null {
  const [windows, setWindows] = useState<WindowView[] | null>(null);

  useEffect(() => {
    let active = true;

    async function refresh(): Promise<void> {
      try {
        const listed = await bridge.listWindows();
        if (active) {
          setWindows(listed);
        }
      } catch {
        // The next refresh retries; the list on screen stays as it was.
      }
    }

    void refresh();
    const interval = setInterval(() => void refresh(), REFRESH_MS);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, []);

  return windows;
}
