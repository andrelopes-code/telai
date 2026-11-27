import { LiveScreen } from "./live/LiveScreen";
import { PickerScreen } from "./picker/PickerScreen";
import { ServerError } from "./ServerError";
import { useLiveSession } from "./use-live-session";

export function App() {
  const session = useLiveSession();

  if (session.status === null) {
    return null;
  }

  if (session.status.serverError !== null) {
    return <ServerError url={session.status.url} />;
  }

  if (session.live !== null) {
    return (
      <LiveScreen
        key={session.live.startedAt}
        live={session.live}
        url={session.status.url}
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
