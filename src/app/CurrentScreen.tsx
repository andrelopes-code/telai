import { FaultScreen } from "./fault/FaultScreen";
import { LiveScreen } from "./live/LiveScreen";
import { PickerScreen } from "./picker/PickerScreen";
import type { AppStatus } from "./types";
import type { LiveSession } from "./use-live-session";

interface CurrentScreenProps {
  session: LiveSession;
  status: AppStatus;
}

export function CurrentScreen({ session, status }: CurrentScreenProps) {
  if (status.serverError !== null) {
    return <FaultScreen url={status.url} onRetry={() => void session.retry()} />;
  }

  if (session.live !== null) {
    return (
      <LiveScreen
        key={session.live.startedAt}
        live={session.live}
        url={status.url}
        onStop={() => void session.stop()}
      />
    );
  }

  return (
    <PickerScreen
      ended={session.ended}
      error={session.error}
      starting={session.starting}
      onStart={(handle) => void session.start(handle)}
    />
  );
}
