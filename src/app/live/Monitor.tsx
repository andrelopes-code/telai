import { useRef } from "react";

import { text } from "../i18n/text";
import type { LiveSnapshot } from "../types";
import { LevelLine } from "./LevelLine";
import { Tally } from "./Tally";
import { usePreview } from "./use-preview";

interface MonitorProps {
  live: LiveSnapshot;
  url: string;
}

/** What the call receives: the captured window, the time on air and the sound level. */
export function Monitor({ live, url }: MonitorProps) {
  const canvas = useRef<HTMLCanvasElement>(null);

  usePreview(url, canvas);

  return (
    <div className="monitor">
      <canvas
        ref={canvas}
        className="monitor-video"
        width={16}
        height={9}
        aria-label={text.previewOf(live.source.title)}
      />

      {live.minimized && (
        <div className="monitor-veil">
          <strong>{text.minimizedTitle}</strong>
          <span>{text.minimizedHint}</span>
        </div>
      )}

      <Tally startedAt={live.startedAt} />
      <LevelLine level={live.level} />
    </div>
  );
}
