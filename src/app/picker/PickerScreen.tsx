import { useState } from "react";

import { pickerNotice } from "../common/error-message";
import { Notice } from "../common/Notice";
import { text } from "../i18n/text";
import type { EndReason, SourceRef } from "../types";
import { filterMonitors, groupWindows, monitorLabel, selectedSource } from "./group-windows";
import { PickerBody } from "./PickerBody";
import { PickerFooter } from "./PickerFooter";
import { SearchField } from "./SearchField";
import type { Sources } from "./use-sources";
import { useSources } from "./use-sources";

interface PickerScreenProps {
  ended: EndReason | null;
  error: unknown;
  starting: boolean;
  onStart: (source: SourceRef) => void;
}

export function PickerScreen({ ended, error, starting, onStart }: PickerScreenProps) {
  const sources = useSources();

  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<SourceRef | null>(null);

  const groups = sources === null ? null : groupWindows(sources.windows, query);
  const monitors = sources === null ? [] : filterMonitors(sources.monitors, query);
  const selected =
    sources === null ? null : selectedSource(sources.windows, sources.monitors, picked);
  const name = sources === null || selected === null ? null : sourceName(sources, selected);
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
          monitors={monitors}
          query={query}
          selected={selected}
          onSelect={setPicked}
          onStart={onStart}
        />
      </div>

      <PickerFooter
        chosen={selected === null || name === null ? null : { source: selected, name }}
        starting={starting}
        onStart={onStart}
      />
    </main>
  );
}

function sourceName(sources: Sources, selected: SourceRef): string | null {
  if (selected.kind === "window") {
    return sources.windows.find((window) => window.handle === selected.handle)?.title ?? null;
  }

  const monitor = sources.monitors.find((item) => item.handle === selected.handle);

  return monitor === undefined ? null : monitorLabel(monitor);
}
