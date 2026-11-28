import type { MonitorView } from "../types";
import { MonitorTile } from "./MonitorTile";

interface MonitorGroupProps {
  label: string;
  monitors: MonitorView[];
  selected: number | null;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function MonitorGroup({ label, monitors, selected, onSelect, onStart }: MonitorGroupProps) {
  if (monitors.length === 0) {
    return null;
  }

  return (
    <section className="group">
      <h2 className="group-label">
        <span className="label">{label}</span>
        <span className="label group-count">{monitors.length}</span>
      </h2>
      <div className="gallery" role="radiogroup" aria-label={label}>
        {monitors.map((monitor) => (
          <MonitorTile
            key={monitor.handle}
            monitor={monitor}
            selected={monitor.handle === selected}
            onSelect={onSelect}
            onStart={onStart}
          />
        ))}
      </div>
    </section>
  );
}
