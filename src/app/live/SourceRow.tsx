import { AppIcon } from "../common/AppIcon";
import { text } from "../i18n/text";
import type { SourceView } from "../types";

interface SourceRowProps {
  source: SourceView;
  onStop: () => void;
}

export function SourceRow({ source, onStop }: SourceRowProps) {
  return (
    <div className="source">
      <span className="source-name" title={source.title}>
        <AppIcon icon={source.icon} />
        <span className="line">{source.title}</span>
      </span>
      <button className="button" type="button" onClick={onStop}>
        <span className="stop" />
        <span className="label">{text.stop}</span>
      </button>
    </div>
  );
}
