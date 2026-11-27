interface NoticeProps {
  children: string;
}

/** One line of status text. Warnings are written out, without an icon. */
export function Notice({ children }: NoticeProps) {
  return (
    <p className="notice" role="status" title={children}>
      <span className="line">{children}</span>
    </p>
  );
}
