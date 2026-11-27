import { english } from "./english";
import { portuguese } from "./portuguese";

/** Strings in the language of the system, which the webview reports. */
export const text = navigator.language.toLowerCase().startsWith("pt") ? portuguese : english;
