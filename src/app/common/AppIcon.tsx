import { AppWindow } from "lucide-react";

import styles from "./AppIcon.module.css";

interface AppIconProps {
  icon: string | null;
}

export function AppIcon({ icon }: AppIconProps) {
  if (icon === null) {
    return <AppWindow className={styles.fallback} size={24} strokeWidth={1.5} aria-hidden />;
  }

  return <img className={styles.icon} src={icon} width={24} height={24} alt="" />;
}
