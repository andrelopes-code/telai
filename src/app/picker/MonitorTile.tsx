import { Check, Monitor } from "lucide-react";
import type { KeyboardEvent } from "react";

import { text } from "../i18n/text";
import type { MonitorView } from "../types";
import { monitorLabel } from "./group-windows";

interface MonitorTileProps {
  monitor: MonitorView;
  selected: boolean;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function MonitorTile({ monitor, selected, onSelect, onStart }: MonitorTileProps) {
  const label = monitorLabel(monitor);
  const size = text.screenSize(monitor.width, monitor.height);

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "Enter") {
      event.preventDefault();
      onStart(monitor.handle);
    }
  }

  return (
    <button
      className="tile"
      type="button"
      role="radio"
      aria-checked={selected}
      title={`${label} (${size})`}
      onClick={() => {
        onSelect(monitor.handle);
      }}
      onDoubleClick={() => {
        onStart(monitor.handle);
      }}
      onKeyDown={handleKeyDown}
    >
      <span className="tile-thumb">
        {monitor.thumbnail !== null && <img src={monitor.thumbnail} alt="" />}

        {monitor.thumbnail === null && (
          <span className="tile-placeholder">
            <Monitor className="app-icon icon" aria-hidden />
            <span className="label">{size}</span>
          </span>
        )}

        {selected && (
          <span className="tile-check">
            <Check className="icon" aria-hidden />
          </span>
        )}
      </span>
      <span className="tile-name">
        <Monitor className="app-icon icon" aria-hidden />
        <span className="line">{label}</span>
      </span>
    </button>
  );
}
