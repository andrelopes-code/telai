import { CircleCheck } from "lucide-react";

import { text } from "../i18n/text";
import type { LiveStatusKind } from "./step-states";

const TITLES: Record<LiveStatusKind, string> = {
  open: text.openTitle,
  mute: text.muteTitle,
  muteAttention: text.muteTitle,
  ready: text.readyTitle,
};

const HINTS: Record<LiveStatusKind, string> = {
  open: text.openHint,
  mute: text.muteHint,
  muteAttention: text.muteHint,
  ready: text.readyHint,
};

const TONES: Partial<Record<LiveStatusKind, string>> = {
  muteAttention: "attention",
  ready: "done",
};

interface LiveStatusProps {
  status: LiveStatusKind;
}

export function LiveStatus({ status }: LiveStatusProps) {
  return (
    <div className="status" role="status">
      <p className="status-title">{TITLES[status]}</p>
      <p className="status-hint" data-tone={TONES[status]}>
        {status === "ready" && <CircleCheck className="icon" aria-hidden />}
        <span className="line">{HINTS[status]}</span>
      </p>
    </div>
  );
}
