import type { WindowView } from "../types";
import { WindowTile } from "./WindowTile";

interface WindowGroupProps {
  label: string;
  windows: WindowView[];
  selected: number | null;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function WindowGroup({ label, windows, selected, onSelect, onStart }: WindowGroupProps) {
  if (windows.length === 0) {
    return null;
  }

  return (
    <section className="group">
      <h2 className="group-label">
        <span className="label">{label}</span>
        <span className="label group-count">{windows.length}</span>
      </h2>
      <div className="gallery" role="radiogroup" aria-label={label}>
        {windows.map((window) => (
          <WindowTile
            key={window.handle}
            window={window}
            selected={window.handle === selected}
            onSelect={onSelect}
            onStart={onStart}
          />
        ))}
      </div>
    </section>
  );
}
