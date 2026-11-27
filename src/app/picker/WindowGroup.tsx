import type { WindowView } from "../types";
import styles from "./WindowGroup.module.css";
import { WindowRow } from "./WindowRow";

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
    <section className={styles.group}>
      <h2 className={styles.label}>{label}</h2>
      {windows.map((window) => (
        <WindowRow
          key={window.handle}
          window={window}
          selected={window.handle === selected}
          onSelect={onSelect}
          onStart={onStart}
        />
      ))}
    </section>
  );
}
