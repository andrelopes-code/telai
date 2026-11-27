import { useEffect, useState } from "react";

import { bridge } from "../bridge";
import type { WindowView } from "../types";

/** Each refresh takes a still of every window, so it runs every 3 seconds. */
const REFRESH_MS = 3000;

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
