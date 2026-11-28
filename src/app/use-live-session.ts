import { useEffect, useState } from "react";

import { bridge } from "./bridge";
import type { AppStatus, EndReason, LiveSnapshot, SourceRef } from "./types";

export interface LiveSession {
  status: AppStatus | null;
  live: LiveSnapshot | null;
  ended: EndReason | null;
  error: unknown;
  starting: boolean;
  start: (source: SourceRef) => Promise<void>;
  stop: () => Promise<void>;
  retry: () => Promise<void>;
}

/** The stream state kept by the app, with the commands that change it. */
export function useLiveSession(): LiveSession {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [live, setLive] = useState<LiveSnapshot | null>(null);
  const [ended, setEnded] = useState<EndReason | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    void bridge.status().then((initial) => {
      setStatus(initial);
      setLive(initial.live);
    });

    const unlistenLive = bridge.onLive(setLive);
    const unlistenEnded = bridge.onEnded((reason) => {
      setLive(null);
      setEnded(reason);
    });

    return () => {
      void unlistenLive.then((unlisten) => {
        unlisten();
      });
      void unlistenEnded.then((unlisten) => {
        unlisten();
      });
    };
  }, []);

  async function start(source: SourceRef): Promise<void> {
    setStarting(true);
    setEnded(null);
    setError(null);

    try {
      setLive(await bridge.start(source));
    } catch (failure: unknown) {
      setError(failure);
    } finally {
      setStarting(false);
    }
  }

  async function stop(): Promise<void> {
    await bridge.stop();
    setLive(null);
  }

  async function retry(): Promise<void> {
    setStatus(await bridge.retryServer());
  }

  return { status, live, ended, error, starting, start, stop, retry };
}
