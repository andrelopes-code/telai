import { Check, Copy, ExternalLink } from "lucide-react";

import { bridge } from "../bridge";
import { text } from "../i18n/text";
import { useCopied } from "./use-copied";

interface LiveFooterProps {
  url: string;
  viewers: number;
}

export function LiveFooter({ url, viewers }: LiveFooterProps) {
  const [copied, copy] = useCopied(url);

  return (
    <footer className="live-foot">
      <span className="link" data-copied={copied || undefined}>
        <span className="link-url line">{copied ? text.linkCopied : url}</span>
        <button
          className="icon-button"
          type="button"
          aria-label={text.copyLink}
          title={text.copyLink}
          onClick={copy}
        >
          {copied && <Check className="icon" aria-hidden />}
          {!copied && <Copy className="icon" aria-hidden />}
        </button>
      </span>
      <button
        className="button"
        data-tone={viewers === 0 ? "primary" : undefined}
        type="button"
        onClick={() => void bridge.openPlayer()}
      >
        <span className="label">{text.openPlayer}</span>
        <ExternalLink className="icon" aria-hidden />
      </button>
    </footer>
  );
}
