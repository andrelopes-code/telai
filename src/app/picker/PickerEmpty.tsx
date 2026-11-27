import { AppWindow, Search } from "lucide-react";

import { text } from "../i18n/text";

interface PickerEmptyProps {
  query: string;
}

export function PickerEmpty({ query }: PickerEmptyProps) {
  const searched = query.trim();

  return (
    <div className="empty">
      <span className="empty-icon">
        {searched === "" && <AppWindow className="icon" aria-hidden />}
        {searched !== "" && <Search className="icon" aria-hidden />}
      </span>
      <p className="empty-title">
        {searched === "" ? text.noWindowsTitle : text.noMatchesTitle(searched)}
      </p>
      <p className="empty-hint">{searched === "" ? text.noWindowsHint : text.noMatchesHint}</p>
    </div>
  );
}
