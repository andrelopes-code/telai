import { bridge } from "../bridge";
import { text } from "../i18n/text";
import styles from "./PlayerLink.module.css";
import { useCopied } from "./use-copied";

interface PlayerLinkProps {
  url: string;
}

export function PlayerLink({ url }: PlayerLinkProps) {
  const [copied, copy] = useCopied(url);

  return (
    <span className={styles.link}>
      <span className={styles.url}>{url}</span>
      <button type="button" className={styles.action} onClick={copy}>
        {copied ? text.copied : text.copy}
      </button>
      <button type="button" className={styles.action} onClick={() => void bridge.openPlayer()}>
        {text.open}
      </button>
    </span>
  );
}
