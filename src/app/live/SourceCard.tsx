import { AppIcon } from "../common/AppIcon";
import type { SourceView } from "../types";
import { LevelMeter } from "./LevelMeter";
import styles from "./SourceCard.module.css";

interface SourceCardProps {
  source: SourceView;
  level: number;
}

export function SourceCard({ source, level }: SourceCardProps) {
  return (
    <section className={styles.card}>
      <div className={styles.source} title={source.title}>
        <AppIcon icon={source.icon} />
        <span className={styles.title}>{source.title}</span>
        <span className={styles.process}>{source.processName}</span>
      </div>
      <LevelMeter level={level} />
    </section>
  );
}
