import type { KeyboardEvent } from "react";

import { AppIcon } from "../common/AppIcon";
import { text } from "../i18n/text";
import type { WindowView } from "../types";
import styles from "./WindowRow.module.css";
import { SoundBars } from "./SoundBars";

interface WindowRowProps {
  window: WindowView;
  selected: boolean;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function WindowRow({ window, selected, onSelect, onStart }: WindowRowProps) {
  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter") {
      onStart(window.handle);
    }
  }

  return (
    <label
      className={styles.row}
      title={window.title}
      onDoubleClick={() => {
        onStart(window.handle);
      }}
    >
      <input
        className={styles.radio}
        type="radio"
        name="window"
        checked={selected}
        onChange={() => {
          onSelect(window.handle);
        }}
        onKeyDown={handleKeyDown}
      />
      <AppIcon icon={window.icon} />
      <span className={styles.title}>{window.title}</span>
      <span className={styles.process}>{window.processName}</span>
      <span className={styles.sound}>
        {window.playingAudio && <SoundBars label={text.playingSound} />}
      </span>
    </label>
  );
}
