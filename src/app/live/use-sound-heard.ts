import { useState } from "react";

/** About -40 dBFS: quieter than speech or music, louder than dither noise. */
const HEARD_LEVEL = 0.01;

/** Whether the window played sound at any time while this hook is mounted. */
export function useSoundHeard(level: number): boolean {
  const [heard, setHeard] = useState(false);

  if (!heard && level > HEARD_LEVEL) {
    setHeard(true);
  }

  return heard;
}
