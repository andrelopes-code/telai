import { Notice } from "../common/Notice";
import { text } from "../i18n/text";
import type { LiveSnapshot } from "../types";
import { LiveHeader } from "./LiveHeader";
import styles from "./LiveScreen.module.css";
import { SourceCard } from "./SourceCard";
import { stepStates } from "./step-states";
import { StepList } from "./StepList";
import { useSoundHeard } from "./use-sound-heard";

interface LiveScreenProps {
  live: LiveSnapshot;
  url: string;
  onStop: () => void;
}

export function LiveScreen({ live, url, onStop }: LiveScreenProps) {
  const soundHeard = useSoundHeard(live.level);

  const steps = stepStates(live, soundHeard);

  return (
    <main className={styles.screen}>
      <LiveHeader startedAt={live.startedAt} onStop={onStop} />
      <SourceCard source={live.source} level={live.level} />
      {live.minimized && <Notice tone="attention">{text.minimized}</Notice>}
      <StepList steps={steps} viewers={live.viewers} url={url} />
    </main>
  );
}
