import { LuCircleCheck, LuChevronLeft } from "react-icons/lu";
import type { HFModel } from "./useModelSearch";
import { formatDownloads } from "./useModelSearch";

interface SearchResultsDropdownProps {
  query: string;
  results: HFModel[];
  selectedIndex: number;
  downloadedModelIds: string[];
  onSelect: (model: HFModel) => void;
  onHover: (index: number) => void;
}

export function SearchResultsDropdown({
  query,
  results,
  selectedIndex,
  downloadedModelIds,
  onSelect,
  onHover,
}: SearchResultsDropdownProps) {
  return (
    <div className="absolute top-full left-0 right-0 bg-slate-grey-950 border border-t-0 border-indigo-smoke-700 rounded-b-md overflow-hidden z-50 shadow-[0_8px_24px_rgba(0,0,0,0.4)]">
      {results.length === 0 ? (
        <div className="px-3 py-2 text-xs font-mono text-slate-grey-600">
          No models found for "{query}"
        </div>
      ) : (
        results.map((m, i) => {
          const alreadyDownloaded = downloadedModelIds.includes(m.id);
          return (
            <button
              key={m.id}
              onClick={() => onSelect(m)}
              onMouseEnter={() => onHover(i)}
              className={`w-full flex items-center gap-3 px-3 py-1.5 text-left transition-colors border-l-2 ${
                i === selectedIndex
                  ? "bg-indigo-smoke-900 border-indigo-smoke-500"
                  : "border-transparent hover:bg-slate-grey-900"
              }`}
            >
              {alreadyDownloaded && (
                <LuCircleCheck
                  size={11}
                  className="text-moss-green-500 shrink-0"
                />
              )}
              <span className="font-mono text-xs text-parchment-200 truncate flex-1 min-w-0">
                {m.id}
              </span>
              <span className="shrink-0 font-mono text-[10px] text-slate-grey-600 tabular-nums w-12 text-right">
                {formatDownloads(m.downloads)}↓
              </span>
              <LuChevronLeft
                size={10}
                className="text-slate-grey-700 rotate-180 shrink-0"
              />
            </button>
          );
        })
      )}

      <div className="flex items-center gap-2 px-3 py-1 border-t border-slate-grey-800">
        <kbd className="px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-500">
          ↑↓
        </kbd>
        <span className="text-[10px] font-mono text-slate-grey-600">
          navigate
        </span>
        <kbd className="px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-500">
          ↵
        </kbd>
        <span className="text-[10px] font-mono text-slate-grey-600">
          expand
        </span>
        <kbd className="ml-auto px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-500">
          esc
        </kbd>
        <span className="text-[10px] font-mono text-slate-grey-600">close</span>
      </div>
    </div>
  );
}
