import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";

import mark from "../shared/mark.svg";
import { text } from "./i18n/text";

export function TitleBar() {
  return (
    <header className="titlebar" data-tauri-drag-region>
      <img className="titlebar-mark" src={mark} alt="" />
      <span className="titlebar-name label" data-tauri-drag-region>
        Telai
      </span>
      <button
        className="titlebar-button"
        type="button"
        aria-label={text.minimize}
        onClick={() => void getCurrentWindow().minimize()}
      >
        <Minus className="icon" aria-hidden />
      </button>
      <button
        className="titlebar-button"
        type="button"
        aria-label={text.close}
        data-close
        onClick={() => void getCurrentWindow().close()}
      >
        <X className="icon" aria-hidden />
      </button>
    </header>
  );
}
