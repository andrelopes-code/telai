import { getCurrentWindow } from "@tauri-apps/api/window";
import { RefreshCw } from "lucide-react";

import { text } from "../i18n/text";

interface FaultScreenProps {
  url: string;
  onRetry: () => void;
}

export function FaultScreen({ url, onRetry }: FaultScreenProps) {
  const address = new URL(url).host;

  return (
    <main className="fault" role="alert">
      <h1 className="fault-title">{text.faultTitle}</h1>
      <p className="fault-hint" data-wrap-ok>
        {text.faultHintBefore}
        <span className="fault-port">{address}</span>
        {text.faultHintAfter}
      </p>
      <div className="fault-actions">
        <button className="button" data-tone="primary" type="button" onClick={onRetry}>
          <RefreshCw className="icon" aria-hidden />
          <span className="label">{text.retry}</span>
        </button>
        <button className="button" type="button" onClick={() => void getCurrentWindow().close()}>
          <span className="label">{text.quit}</span>
        </button>
      </div>
    </main>
  );
}
