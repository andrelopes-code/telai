import { text } from "../i18n/text";
import type { WindowGroups } from "./group-windows";
import { WindowGroup } from "./WindowGroup";
import styles from "./WindowList.module.css";

interface WindowListProps {
  groups: WindowGroups | null;
  query: string;
  selected: number | null;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function WindowList({ groups, query, selected, onSelect, onStart }: WindowListProps) {
  if (groups === null) {
    return <div className={styles.list} />;
  }

  const empty = groups.playing.length === 0 && groups.others.length === 0;
  const emptyMessage = query.trim() === "" ? text.noWindows : text.noMatches(query.trim());

  return (
    <div className={styles.list} role="radiogroup" aria-label={text.pickerTitle}>
      {empty && <p className={styles.empty}>{emptyMessage}</p>}

      <WindowGroup
        label={text.groupPlaying}
        windows={groups.playing}
        selected={selected}
        onSelect={onSelect}
        onStart={onStart}
      />

      <WindowGroup
        label={text.groupOthers}
        windows={groups.others}
        selected={selected}
        onSelect={onSelect}
        onStart={onStart}
      />
    </div>
  );
}
