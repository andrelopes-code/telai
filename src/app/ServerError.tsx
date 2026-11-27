import { text } from "./i18n/text";
import styles from "./ServerError.module.css";

interface ServerErrorProps {
  url: string;
}

export function ServerError({ url }: ServerErrorProps) {
  const port = new URL(url).port;

  return (
    <main className={styles.screen} role="alert">
      <h1 className={styles.title}>{text.serverErrorTitle(port)}</h1>
      <p className={styles.hint}>{text.serverErrorHint}</p>
    </main>
  );
}
