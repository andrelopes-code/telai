import { text } from "../i18n/text";
import { PlayerLink } from "./PlayerLink";
import { Step } from "./Step";
import type { StepStates } from "./step-states";
import styles from "./StepList.module.css";

interface StepListProps {
  steps: StepStates;
  viewers: number;
  url: string;
}

export function StepList({ steps, viewers, url }: StepListProps) {
  const openTitle = steps.open === "done" ? text.openStepDone(viewers) : text.openStepTitle;
  const muteTitle = steps.mute === "done" ? text.muteStepDone : text.muteStepTitle;
  const muteDetail = steps.mute === "attention" ? text.muteStepAttention : text.muteStepHint;

  return (
    <section className={styles.section}>
      <h2 className={styles.title}>{text.stepsTitle}</h2>

      <ol className={styles.list}>
        <Step number={1} state={steps.open} title={openTitle} detail={<PlayerLink url={url} />} />
        <Step number={2} state={steps.mute} title={muteTitle} detail={muteDetail} />
        <Step
          number={3}
          state={steps.share}
          title={text.shareStepTitle}
          detail={text.shareStepHint}
        />
      </ol>
    </section>
  );
}
