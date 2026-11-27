import type { ReactNode } from "react";

import { TitleBar } from "./TitleBar";

interface AppWindowProps {
  live: boolean;
  children: ReactNode;
}

export function AppWindow({ live, children }: AppWindowProps) {
  return (
    <div className="window" data-live={live || undefined}>
      <TitleBar />
      {children}
    </div>
  );
}
