import { Search } from "lucide-react";

import { text } from "../i18n/text";

interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
}

export function SearchField({ value, onChange }: SearchFieldProps) {
  return (
    <label className="search">
      <Search className="icon" aria-hidden />
      <input
        className="search-input"
        type="search"
        value={value}
        placeholder={text.searchPlaceholder}
        aria-label={text.searchPlaceholder}
        spellCheck={false}
        onChange={(event) => {
          onChange(event.target.value);
        }}
      />
    </label>
  );
}
