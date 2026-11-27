import { Check } from "lucide-react";
import type { ReactNode } from "react";

import styles from "./Step.module.css";
import type { StepState } from "./step-states";

interface StepProps {
  number: number;
  state: StepState;
  title: string;
  detail: ReactNode;
}

export function Step({ number, state, title, detail }: StepProps) {
  return (
    <li className={styles.step} data-state={state}>
      <span className={styles.marker} aria-hidden>
        {state === "done" && <Check size={14} strokeWidth={3} />}
        {state !== "done" && number}
      </span>
      <span className={styles.title}>{title}</span>
      <span className={styles.detail}>{detail}</span>
    </li>
  );
}
