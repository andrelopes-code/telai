import styles from "./SoundBars.module.css";

interface SoundBarsProps {
  label: string;
}

export function SoundBars({ label }: SoundBarsProps) {
  return (
    <span className={styles.bars} role="img" aria-label={label} title={label}>
      <span className={styles.bar} />
      <span className={styles.bar} />
      <span className={styles.bar} />
    </span>
  );
}
