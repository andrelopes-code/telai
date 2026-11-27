import { Check } from "lucide-react";

import { text } from "../i18n/text";
import type { StepState } from "./step-states";

interface LiveStepsProps {
  steps: StepState[];
  viewers: number;
}

export function LiveSteps({ steps, viewers }: LiveStepsProps) {
  const names = [
    viewers > 1 ? text.stepOpenMany(viewers) : text.stepOpen,
    text.stepMute,
    text.stepShare,
  ];

  return (
    <ol className="steps">
      {steps.map((state, index) => (
        <li key={names[index] ?? index} className="step" data-state={state}>
          <span className="step-bar" />
          <span className="step-name">
            {state === "done" && <Check className="icon" aria-hidden />}
            <span className="line">{names[index]}</span>
          </span>
        </li>
      ))}
    </ol>
  );
}
