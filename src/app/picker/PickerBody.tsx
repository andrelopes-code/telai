import { text } from "../i18n/text";
import { GallerySkeleton } from "./GallerySkeleton";
import type { WindowGroups } from "./group-windows";
import { PickerEmpty } from "./PickerEmpty";
import { WindowGroup } from "./WindowGroup";

interface PickerBodyProps {
  groups: WindowGroups | null;
  query: string;
  selected: number | null;
  onSelect: (handle: number) => void;
  onStart: (handle: number) => void;
}

export function PickerBody({ groups, query, selected, onSelect, onStart }: PickerBodyProps) {
  if (groups === null) {
    return <GallerySkeleton />;
  }

  if (groups.playing.length === 0 && groups.others.length === 0) {
    return <PickerEmpty query={query} />;
  }

  return (
    <>
      <WindowGroup
        label={text.groupPlaying}
        windows={groups.playing}
        selected={selected}
        onSelect={onSelect}
        onStart={onStart}
      />

      <WindowGroup
        label={text.groupOthers}
        windows={groups.others}
        selected={selected}
        onSelect={onSelect}
        onStart={onStart}
      />
    </>
  );
}
