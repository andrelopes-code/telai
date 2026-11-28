import { useState } from "react";

import { bridge } from "../bridge";
import { AppIcon } from "../common/AppIcon";
import { text } from "../i18n/text";
import type { AudioAppView } from "../types";
import { mixRows, volumePercent } from "./audio-mix";
import { levelPercent, levelZone } from "./level";

interface AudioMixProps {
  apps: AudioAppView[];
}

/** The apps with sound during a screen stream: which ones the call hears, and how loud. */
export function AudioMix({ apps }: AudioMixProps) {
  return (
    <section className="mix" aria-label={text.audioMixTitle}>
      <h2 className="mix-title label">{text.audioMixTitle}</h2>
      {apps.length === 0 && <p className="mix-empty">{text.audioMixEmpty}</p>}
      {apps.length > 0 && (
        <ul className="mix-list">
          {mixRows(apps).map((app) => (
            <AudioMixRow key={app.processId} app={app} />
          ))}
        </ul>
      )}
    </section>
  );
}

function AudioMixRow({ app }: { app: AudioAppView }) {
  // While the slider is held, it shows its own value instead of the one in
  // the snapshots, which arrive ten times a second.
  const [draft, setDraft] = useState<number | null>(null);
  const percent = draft ?? volumePercent(app.volume);
  const level = levelPercent(app.level);

  function save(included: boolean, volume: number) {
    void bridge.setAudioApp(app.name, included, volume);
  }

  function release() {
    setDraft(null);
  }

  return (
    <li className="mix-row" data-included={app.included || undefined}>
      <input
        className="mix-check"
        type="checkbox"
        checked={app.included}
        aria-label={text.includeApp(app.name)}
        onChange={(event) => {
          save(event.currentTarget.checked, app.volume);
        }}
      />
      <span className="mix-name" title={app.name}>
        <AppIcon icon={app.icon} />
        <span className="line">{app.name}</span>
      </span>
      <span
        className="mix-level"
        role="meter"
        aria-label={text.soundLevel}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(level)}
        data-zone={levelZone(level)}
      >
        <i style={{ inlineSize: `${level.toFixed(1)}%` }} />
      </span>
      <input
        className="mix-volume"
        type="range"
        min={0}
        max={100}
        step={1}
        value={percent}
        disabled={!app.included}
        aria-label={text.volumeOf(app.name)}
        onChange={(event) => {
          const value = Number(event.currentTarget.value);
          setDraft(value);
          save(app.included, value / 100);
        }}
        onPointerUp={release}
        onKeyUp={release}
        onBlur={release}
      />
      <span className="mix-percent">{percent}%</span>
    </li>
  );
}
