import { useEffect, useState } from "react";

import { bridge } from "../bridge";
import type { MonitorView, WindowView } from "../types";

/** Each refresh takes a still of every window and screen, so it runs every 3 seconds. */
const REFRESH_MS = 3000;

export interface Sources {
  windows: WindowView[];
  monitors: MonitorView[];
}

/** The capturable windows and screens, refreshed while the picker is mounted. */
export function useSources(): Sources | null {
  const [sources, setSources] = useState<Sources | null>(null);

  useEffect(() => {
    let active = true;

    async function refresh(): Promise<void> {
      try {
        const [windows, monitors] = await Promise.all([
          bridge.listWindows(),
          bridge.listMonitors(),
        ]);
        if (active) {
          setSources({ windows, monitors });
        }
      } catch {
        // The next refresh retries; the lists on screen stay as they were.
      }
    }

    void refresh();
    const interval = setInterval(() => void refresh(), REFRESH_MS);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, []);

  return sources;
}
