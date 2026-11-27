import type { KeyboardEvent } from "react";

import { AppIcon } from "../common/AppIcon";
import type { WindowView } from "../types";
import { TileThumb } from "./TileThumb";

interface WindowTileProps {
  window: WindowView;
  selected: boolean;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function WindowTile({ window, selected, onSelect, onStart }: WindowTileProps) {
  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "Enter") {
      event.preventDefault();
      onStart(window.handle);
    }
  }

  return (
    <button
      className="tile"
      type="button"
      role="radio"
      aria-checked={selected}
      title={`${window.title} (${window.processName})`}
      onClick={() => {
        onSelect(window.handle);
      }}
      onDoubleClick={() => {
        onStart(window.handle);
      }}
      onKeyDown={handleKeyDown}
    >
      <TileThumb window={window} selected={selected} />
      <span className="tile-name">
        <AppIcon icon={window.icon} />
        <span className="line">{window.title}</span>
      </span>
    </button>
  );
}
