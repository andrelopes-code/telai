import { Square } from "lucide-react";

import button from "../common/button.module.css";
import { text } from "../i18n/text";
import { formatElapsed } from "./format-elapsed";
import styles from "./LiveHeader.module.css";
import { useNow } from "./use-now";

interface LiveHeaderProps {
  startedAt: number;
  onStop: () => void;
}

export function LiveHeader({ startedAt, onStop }: LiveHeaderProps) {
  const now = useNow(1000);

  const elapsed = formatElapsed(now - startedAt);

  return (
    <header className={styles.header}>
      <span className={styles.tally}>
        <span className={styles.light} aria-hidden />
        {text.live}
      </span>
      <time className={styles.elapsed}>{elapsed}</time>
      <button type="button" className={button.secondary} onClick={onStop}>
        <Square size={12} fill="currentColor" aria-hidden />
        {text.stop}
      </button>
    </header>
  );
}
