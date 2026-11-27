import { text } from "../i18n/text";
import { formatElapsed } from "./format-elapsed";
import { useNow } from "./use-now";

interface TallyProps {
  startedAt: number;
}

export function Tally({ startedAt }: TallyProps) {
  const now = useNow(1000);

  const elapsed = formatElapsed(now - startedAt);

  return (
    <span className="tally">
      <span className="tally-light" />
      <span className="label">{text.live}</span>
      <span className="label tally-time">{elapsed}</span>
    </span>
  );
}
