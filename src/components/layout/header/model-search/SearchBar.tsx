import {
  LuSearch,
  LuLoaderCircle,
  LuX,
  LuChevronLeft,
  LuDownload,
} from "react-icons/lu";
import type { RefObject } from "react";
import type { HFModel } from "./useModelSearch";

interface SearchBarProps {
  isOpen: boolean;
  isSearching: boolean;
  expandedModel: HFModel | null;
  query: string;
  inputRef: RefObject<HTMLInputElement | null>;
  onOpen: () => void;
  onClose: () => void;
  onBack: () => void;
  onQueryChange: (q: string) => void;
  onClearQuery: () => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
}

export function SearchBar({
  isOpen,
  isSearching,
  expandedModel,
  query,
  inputRef,
  onOpen,
  onClose,
  onBack,
  onQueryChange,
  onClearQuery,
  onKeyDown,
}: SearchBarProps) {
  // ── Closed trigger ────────────────────────────────────────────────────────
  if (!isOpen) {
    return (
      <button
        onClick={onOpen}
        className="w-full flex items-center gap-2 h-6 bg-slate-grey-950 border border-slate-grey-800 rounded-md px-3 hover:border-slate-grey-600 hover:bg-slate-grey-900 transition-all duration-150 group"
      >
        <LuDownload
          size={11}
          className="text-slate-grey-600 group-hover:text-indigo-smoke-400 transition-colors shrink-0"
        />
        <span className="font-mono text-[11px] leading-none translate-y-px text-slate-grey-600 group-hover:text-slate-grey-400 transition-colors">
          Download a model…
        </span>
      </button>
    );
  }

  // ── Open input bar ────────────────────────────────────────────────────────
  return (
    <div className="flex items-center gap-2 h-6 bg-slate-grey-950 border border-indigo-smoke-700 rounded-md px-2.5">
      {expandedModel ? (
        <button
          onClick={onBack}
          className="text-indigo-smoke-400 hover:text-parchment-300 transition-colors shrink-0"
          title="Back to search"
        >
          <LuChevronLeft size={12} />
        </button>
      ) : isSearching ? (
        <LuLoaderCircle
          size={12}
          className="text-indigo-smoke-400 animate-spin shrink-0"
        />
      ) : (
        <LuSearch size={12} className="text-indigo-smoke-400 shrink-0" />
      )}

      {expandedModel ? (
        <span className="flex-1 font-mono text-xs text-parchment-300 truncate">
          {expandedModel.id}
        </span>
      ) : (
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Search models…"
          className="flex-1 bg-transparent font-mono text-xs text-parchment-200 placeholder-slate-grey-600 outline-none"
        />
      )}

      {!expandedModel && query && (
        <button
          onClick={onClearQuery}
          className="text-slate-grey-600 hover:text-slate-grey-400 transition-colors"
        >
          <LuX size={11} />
        </button>
      )}

      <button
        onClick={onClose}
        className="text-slate-grey-700 hover:text-slate-grey-400 transition-colors shrink-0"
      >
        <LuX size={11} />
      </button>
    </div>
  );
}
