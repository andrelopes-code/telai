import { AppWindow } from "./AppWindow";
import { CurrentScreen } from "./CurrentScreen";
import { useBackdrop } from "./use-backdrop";
import { useLiveSession } from "./use-live-session";

export function App() {
  const session = useLiveSession();

  useBackdrop(session.status?.backdrop);

  if (session.status === null) {
    return null;
  }

  return (
    <AppWindow live={session.live !== null}>
      <CurrentScreen session={session} status={session.status} />
    </AppWindow>
  );
}
