import { AppWindow } from "lucide-react";

interface AppIconProps {
  icon: string | null;
}

/** The program's own icon, or a window glyph when it has none. */
export function AppIcon({ icon }: AppIconProps) {
  if (icon === null) {
    return <AppWindow className="app-icon icon" aria-hidden />;
  }

  return <img className="app-icon" src={icon} alt="" />;
}
