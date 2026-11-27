import { text } from "../i18n/text";
import type { WindowView } from "../types";

interface PickerFooterProps {
  chosen: WindowView | null;
  starting: boolean;
  onStart: (handle: number) => void;
}

export function PickerFooter({ chosen, starting, onStart }: PickerFooterProps) {
  function handleStart() {
    if (chosen) {
      onStart(chosen.handle);
    }
  }

  return (
    <footer className="picker-foot">
      <span className="picker-choice">
        <span className="picker-choice-caption">
          {chosen ? text.selectedCaption : text.noSelectionCaption}
        </span>
        <span className="picker-choice-name line">
          {chosen ? chosen.title : text.noSelectionName}
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
