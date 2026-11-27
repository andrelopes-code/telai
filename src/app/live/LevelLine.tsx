import { text } from "../i18n/text";
import { levelPercent, levelZone } from "./level";

interface LevelLineProps {
  level: number;
}

export function LevelLine({ level }: LevelLineProps) {
  const percent = levelPercent(level);

  return (
    <div
      className="monitor-level"
      role="meter"
      aria-label={text.soundLevel}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(percent)}
      data-zone={levelZone(percent)}
    >
      <i style={{ inlineSize: `${percent.toFixed(1)}%` }} />
    </div>
  );
}
