import { useEffect, useState } from "react";

/** How long the button says "Copied" after a copy. */
const CONFIRMATION_MS = 1500;

/** Copies text to the clipboard and reports the confirmation state. */
export function useCopied(value: string): [boolean, () => void] {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) {
      return;
    }

    const timeout = setTimeout(() => {
      setCopied(false);
    }, CONFIRMATION_MS);

    return () => {
      clearTimeout(timeout);
    };
  }, [copied]);

  function copy(): void {
    void navigator.clipboard.writeText(value).then(() => {
      setCopied(true);
    });
  }

  return [copied, copy];
}
