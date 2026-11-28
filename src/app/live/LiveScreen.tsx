import type { LiveSnapshot } from "../types";
import { AudioMix } from "./AudioMix";
import { LiveFooter } from "./LiveFooter";
import { LiveStatus } from "./LiveStatus";
import { LiveSteps } from "./LiveSteps";
import { Monitor } from "./Monitor";
import { SourceRow } from "./SourceRow";
import { liveStatus, liveSteps } from "./step-states";
import { useSoundHeard } from "./use-sound-heard";

interface LiveScreenProps {
  live: LiveSnapshot;
  url: string;
  onStop: () => void;
}

export function LiveScreen({ live, url, onStop }: LiveScreenProps) {
  const soundHeard = useSoundHeard(live.level);

  const steps = liveSteps({
    viewers: live.viewers,
    browserAudible: live.browserAudible,
    soundHeard,
  });
  const status = liveStatus(steps);

  return (
    <main className="live">
      <LiveStatus status={status} />
      <LiveSteps steps={steps} viewers={live.viewers} />
      <Monitor live={live} url={url} />
      {live.audioApps !== null && <AudioMix apps={live.audioApps} />}
      <SourceRow source={live.source} onStop={onStop} />
      <LiveFooter url={url} viewers={live.viewers} />
    </main>
  );
}
