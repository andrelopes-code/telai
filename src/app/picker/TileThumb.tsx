import { Check } from "lucide-react";

import { AppIcon } from "../common/AppIcon";
import { text } from "../i18n/text";
import type { WindowView } from "../types";

interface TileThumbProps {
  window: WindowView;
  selected: boolean;
}

export function TileThumb({ window, selected }: TileThumbProps) {
  return (
    <span className="tile-thumb">
      {window.thumbnail !== null && <img src={window.thumbnail} alt="" />}

      {window.thumbnail === null && (
        <span className="tile-placeholder">
          <AppIcon icon={window.icon} />
          <span className="label">{text.minimized}</span>
        </span>
      )}

      {window.playingAudio && (
        <span className="tile-sound">
          <span className="bars" aria-hidden>
            <i />
            <i />
            <i />
          </span>
          <span className="label">{text.playing}</span>
        </span>
      )}

      {selected && (
        <span className="tile-check">
          <Check className="icon" aria-hidden />
        </span>
      )}
    </span>
  );
}
