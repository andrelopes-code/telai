import { text } from "../i18n/text";
import type { MonitorView, SourceRef } from "../types";
import { GallerySkeleton } from "./GallerySkeleton";
import type { WindowGroups } from "./group-windows";
import { MonitorGroup } from "./MonitorGroup";
import { PickerEmpty } from "./PickerEmpty";
import { WindowGroup } from "./WindowGroup";

interface PickerBodyProps {
  groups: WindowGroups | null;
  monitors: MonitorView[];
  query: string;
  selected: SourceRef | null;
  onSelect: (source: SourceRef) => void;
  onStart: (source: SourceRef) => void;
}

export function PickerBody({
  groups,
  monitors,
  query,
  selected,
  onSelect,
  onStart,
}: PickerBodyProps) {
  if (groups === null) {
    return <GallerySkeleton />;
  }

  if (groups.playing.length === 0 && groups.others.length === 0 && monitors.length === 0) {
    return <PickerEmpty query={query} />;
  }

  const selectedWindow = selected?.kind === "window" ? selected.handle : null;
  const selectedMonitor = selected?.kind === "monitor" ? selected.handle : null;

  function selectWindow(handle: number) {
    onSelect({ kind: "window", handle });
  }

  function startWindow(handle: number) {
    onStart({ kind: "window", handle });
  }

  return (
    <>
      <WindowGroup
        label={text.groupPlaying}
        windows={groups.playing}
        selected={selectedWindow}
        onSelect={selectWindow}
        onStart={startWindow}
      />

      <MonitorGroup
        label={text.groupScreens}
        monitors={monitors}
        selected={selectedMonitor}
        onSelect={(handle) => {
          onSelect({ kind: "monitor", handle });
        }}
        onStart={(handle) => {
          onStart({ kind: "monitor", handle });
        }}
      />

      <WindowGroup
        label={text.groupOthers}
        windows={groups.others}
        selected={selectedWindow}
        onSelect={selectWindow}
        onStart={startWindow}
      />
    </>
  );
}
