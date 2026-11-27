import { useState } from "react";

import { pickerNotice } from "../common/error-message";
import { Notice } from "../common/Notice";
import { text } from "../i18n/text";
import type { EndReason } from "../types";
import { groupWindows, selectedWindow } from "./group-windows";
import { PickerBody } from "./PickerBody";
import { PickerFooter } from "./PickerFooter";
import { SearchField } from "./SearchField";
import { useWindows } from "./use-windows";

interface PickerScreenProps {
  ended: EndReason | null;
  error: unknown;
  starting: boolean;
  onStart: (handle: number) => void;
}

export function PickerScreen({ ended, error, starting, onStart }: PickerScreenProps) {
  const windows = useWindows();

  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<number | null>(null);

  const groups = windows === null ? null : groupWindows(windows, query);
  const selected = windows === null ? null : selectedWindow(windows, picked);
  const chosen = windows?.find((window) => window.handle === selected) ?? null;
  const notice = pickerNotice(error, ended);

  return (
    <main className="picker">
      <div className="picker-head">
        <h1 className="picker-title">{text.pickerTitle}</h1>
        <SearchField value={query} onChange={setQuery} />
      </div>

      {notice !== null && <Notice>{notice}</Notice>}

      <div className="picker-body">
        <PickerBody
          groups={groups}
          query={query}
          selected={selected}
          onSelect={setPicked}
          onStart={onStart}
        />
      </div>

      <PickerFooter chosen={chosen} starting={starting} onStart={onStart} />
    </main>
  );
}
