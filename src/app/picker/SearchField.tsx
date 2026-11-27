import { Search } from "lucide-react";

import { text } from "../i18n/text";
import styles from "./SearchField.module.css";

interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
}

export function SearchField({ value, onChange }: SearchFieldProps) {
  return (
    <label className={styles.field}>
      <Search className={styles.icon} size={16} aria-hidden />
      <input
        className={styles.input}
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
