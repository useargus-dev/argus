import { Search } from "lucide-react";

import {
  collectFilterOptions,
  SECRET_TYPE_OPTIONS,
} from "../../lib/secret-utils";
import type { SecretMeta, SecretType } from "../../types/secret";
import { MultiSelectFilter } from "./multi-select-filter";

export interface VaultFilterState {
  query: string;
  types: SecretType[];
  environment: string;
  tags: string[];
}

interface VaultFiltersBarProps {
  secrets: SecretMeta[];
  filters: VaultFilterState;
  onChange: (next: VaultFilterState) => void;
}

export function VaultFiltersBar({
  secrets,
  filters,
  onChange,
}: VaultFiltersBarProps) {
  const { environments, tags } = collectFilterOptions(secrets);

  return (
    <div className="mb-4 flex flex-wrap items-center gap-2">
      <div className="relative min-w-[12rem] flex-1 max-w-md">
        <Search
          className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-muted"
          aria-hidden
        />
        <input
          type="search"
          placeholder="Search secrets…"
          value={filters.query}
          onChange={(e) => onChange({ ...filters, query: e.target.value })}
          className="h-9 w-full rounded-md border border-border bg-surface pl-9 pr-3 text-sm placeholder:text-text-muted focus:border-accent focus:outline-none"
        />
      </div>

      <MultiSelectFilter
        label="types"
        options={SECRET_TYPE_OPTIONS}
        selected={filters.types}
        onChange={(types) =>
          onChange({ ...filters, types: types as SecretType[] })
        }
      />

      <select
        value={filters.environment}
        onChange={(e) => onChange({ ...filters, environment: e.target.value })}
        className="h-9 rounded-md border border-border bg-surface px-3 text-sm focus:border-accent focus:outline-none"
        aria-label="Filter by environment"
      >
        <option value="all">All environments</option>
        {environments.map((env) => (
          <option key={env} value={env}>
            {env}
          </option>
        ))}
      </select>

      <MultiSelectFilter
        label="tags"
        options={tags.map((t) => ({ value: t, label: `#${t}` }))}
        selected={filters.tags}
        onChange={(tags) => onChange({ ...filters, tags })}
      />
    </div>
  );
}
