import { useState } from "react";

import button from "../common/button.module.css";
import { pickerNotice } from "../common/error-message";
import { Notice } from "../common/Notice";
import { text } from "../i18n/text";
import type { EndReason } from "../types";
import { groupWindows } from "./group-windows";
import styles from "./PickerScreen.module.css";
import { SearchField } from "./SearchField";
import { useWindows } from "./use-windows";
import { WindowList } from "./WindowList";

interface PickerScreenProps {
  ended: EndReason | null;
  error: unknown;
  starting: boolean;
  onStart: (handle: number) => void;
}

export function PickerScreen({ ended, error, starting, onStart }: PickerScreenProps) {
  const windows = useWindows();

  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<number | null>(null);

  const groups = windows === null ? null : groupWindows(windows, query);
  const notice = pickerNotice(error, ended);
  const canStart = !starting && windows?.some((window) => window.handle === selected) === true;

  function handleStart() {
    if (selected !== null) {
      onStart(selected);
    }
  }

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <h1 className={styles.title}>{text.pickerTitle}</h1>
        <p className={styles.subtitle}>{text.pickerSubtitle}</p>
      </header>

      <SearchField value={query} onChange={setQuery} />
      {notice !== null && <Notice tone="attention">{notice}</Notice>}

      <WindowList
        groups={groups}
        query={query}
        selected={selected}
        onSelect={setSelected}
        onStart={onStart}
      />

      <footer className={styles.footer}>
        <button
          type="button"
          className={[button.primary, button.wide].join(" ")}
          disabled={!canStart}
          onClick={handleStart}
        >
          {starting ? text.starting : text.start}
        </button>
      </footer>
    </main>
  );
}
