import { text } from "../i18n/text";
import styles from "./LevelMeter.module.css";
import { litSegments, segmentTone } from "./meter-segments";

/** 2 dB per segment across the 60 dB scale. */
const SEGMENTS = 30;

const TONES = Array.from({ length: SEGMENTS }, (_, index) => segmentTone(index, SEGMENTS));

interface LevelMeterProps {
  level: number;
}

export function LevelMeter({ level }: LevelMeterProps) {
  const lit = litSegments(level, SEGMENTS);

  return (
    <div
      className={styles.meter}
      role="meter"
      aria-label={text.soundLevel}
      aria-valuemin={0}
      aria-valuemax={SEGMENTS}
      aria-valuenow={lit}
    >
      {TONES.map((tone, index) => (
        <span
          // The segments never move, so the position identifies each one.
          key={index}
          className={styles.segment}
          data-tone={tone}
          data-lit={index < lit}
        />
      ))}
    </div>
  );
}
