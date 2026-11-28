import { text } from "../i18n/text";
import type { SourceRef } from "../types";

interface PickerFooterProps {
  chosen: { source: SourceRef; name: string } | null;
  starting: boolean;
  onStart: (source: SourceRef) => void;
}

export function PickerFooter({ chosen, starting, onStart }: PickerFooterProps) {
  function handleStart() {
    if (chosen) {
      onStart(chosen.source);
    }
  }

  return (
    <footer className="picker-foot">
      <span className="picker-choice">
        <span className="picker-choice-caption">
          {chosen ? text.selectedCaption : text.noSelectionCaption}
        </span>
        <span className="picker-choice-name line">
          {chosen ? chosen.name : text.noSelectionName}
        </span>
      </span>
      <button
        className="button"
        data-tone="live"
        type="button"
        disabled={chosen === null || starting}
        onClick={handleStart}
      >
        {starting && <span className="spinner" />}
        {!starting && <span className="rec" />}
        <span className="label">{starting ? text.starting : text.start}</span>
      </button>
    </footer>
  );
}
