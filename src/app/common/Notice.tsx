import styles from "./Notice.module.css";

interface NoticeProps {
  tone: "attention" | "neutral";
  children: string;
}

/** One line of status text. Warnings are written out, never shown as icons. */
export function Notice({ tone, children }: NoticeProps) {
  return (
    <p className={[styles.notice, styles[tone]].join(" ")} role="status" title={children}>
      {children}
    </p>
  );
}
