import { useState, useRef, useEffect, useCallback } from "react";
import { Search, Loader2, X, ChevronDown } from "lucide-react";

interface HFModel {
  id: string;
  likes: number;
  downloads: number;
  pipeline_tag?: string;
}

interface ModelSearchProps {
  model: string;
  onModelChange: (model: string) => void;
}

// Keywords that identify coding-focused models
const CODER_KEYWORDS = [
  "code",
  "coder",
  "codegen",
  "coding",
  "starcoder",
  "deepseek-coder",
  "wizardcoder",
  "wizard-coder",
  "phind",
  "codellama",
  "code-llama",
  "magicoder",
  "opencoder",
  "qwen-coder",
  "qwencoder",
  "granite-code",
  "codegemma",
  "codestral",
  "devstral",
  "codeqwen",
  "code-qwen",
];

function isCoderModel(id: string): boolean {
  const lower = id.toLowerCase();
  return CODER_KEYWORDS.some((kw) => lower.includes(kw));
}

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

async function fetchModels(query: string): Promise<HFModel[]> {
  // Bias the search toward coding models and restrict to text-generation pipeline
  const codingQuery = `${query} code`;
  const res = await fetch(
    `https://huggingface.co/api/models?search=${encodeURIComponent(codingQuery)}&pipeline_tag=text-generation&limit=20&sort=downloads&direction=-1`,
  );
  const all: HFModel[] = await res.json();
  // Client-side filter to only surface models with coder-specific names
  const coderOnly = all.filter((m) => isCoderModel(m.id));
  // Fall back to the full set if the filter leaves nothing (e.g. very niche query)
  return (coderOnly.length > 0 ? coderOnly : all).slice(0, 8);
}

export function ModelSearch({ model, onModelChange }: ModelSearchProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<HFModel[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);

  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const modelLoaded = model.length > 0;

  const search = useCallback(async (q: string) => {
    if (!q.trim()) {
      setResults([]);
      return;
    }
    setIsLoading(true);
    try {
      setResults(await fetchModels(q));
    } catch {
      setResults([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => search(query), 300);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [query, search]);

  const open = () => {
    setIsOpen(true);
    setQuery("");
    setResults([]);
    setSelectedIndex(-1);
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  const close = () => {
    setIsOpen(false);
    setQuery("");
    setResults([]);
    setSelectedIndex(-1);
  };

  const select = (id: string) => {
    onModelChange(id);
    close();
  };

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        close();
      }
    };
    if (isOpen) document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [isOpen]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      close();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, -1));
    } else if (e.key === "Enter" && selectedIndex >= 0) {
      select(results[selectedIndex].id);
    }
  };

  const showDropdown = isOpen && (results.length > 0 || (query && !isLoading));

  return (
    <div
      ref={containerRef}
      className="relative flex-1 max-w-[480px]"
      style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
    >
      {/* Closed: trigger bar */}
      {!isOpen ? (
        <button
          onClick={open}
          className="w-full flex items-center gap-2 h-6 bg-slate-grey-950 border border-slate-grey-800 rounded-md px-3 hover:border-slate-grey-600 hover:bg-slate-grey-900 transition-all duration-150 group"
        >
          <div
            className={`w-1.5 h-1.5 rounded-full flex-shrink-0 transition-all duration-300 ${
              modelLoaded
                ? "bg-moss-green-500 shadow-[0_0_5px_rgba(115,155,115,0.5)]"
                : "bg-slate-grey-700"
            }`}
          />
          <span
            className={`font-mono text-xs flex-1 text-left truncate ${
              modelLoaded ? "text-parchment-300" : "text-slate-grey-500"
            }`}
          >
            {modelLoaded ? model : "Search coder models…"}
          </span>
          <div className="flex items-center gap-1.5 flex-shrink-0">
            {modelLoaded && (
              <ChevronDown
                size={11}
                className="text-slate-grey-600 group-hover:text-slate-grey-400 transition-colors"
              />
            )}
            <kbd className="hidden group-hover:inline-flex items-center px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-400">
              ⌘P
            </kbd>
          </div>
        </button>
      ) : (
        /* Open: search input */
        <div className="w-full">
          <div className="flex items-center gap-2 h-6 bg-slate-grey-950 border border-indigo-smoke-700 rounded-t-md px-3 shadow-[0_0_0_1px_rgba(114,139,180,0.15)]">
            {isLoading ? (
              <Loader2
                size={12}
                className="text-indigo-smoke-400 animate-spin flex-shrink-0"
              />
            ) : (
              <Search
                size={12}
                className="text-indigo-smoke-400 flex-shrink-0"
              />
            )}
            <input
              ref={inputRef}
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setSelectedIndex(-1);
              }}
              onKeyDown={handleKeyDown}
              placeholder="Search coder models…"
              className="flex-1 bg-transparent font-mono text-xs text-parchment-200 placeholder-slate-grey-600 outline-none"
            />
            {query && (
              <button
                onClick={() => {
                  setQuery("");
                  setResults([]);
                  inputRef.current?.focus();
                }}
                className="text-slate-grey-600 hover:text-slate-grey-400 transition-colors"
              >
                <X size={11} />
              </button>
            )}
          </div>

          {/* Dropdown */}
          {showDropdown && (
            <div className="absolute top-full left-0 right-0 bg-slate-grey-950 border border-t-0 border-indigo-smoke-700 rounded-b-md overflow-hidden z-50 shadow-[0_8px_24px_rgba(0,0,0,0.4)]">
              {results.length === 0 ? (
                <div className="px-3 py-2 text-xs font-mono text-slate-grey-600">
                  No coder models found for "{query}"
                </div>
              ) : (
                results.map((m, i) => (
                  <button
                    key={m.id}
                    onClick={() => select(m.id)}
                    onMouseEnter={() => setSelectedIndex(i)}
                    className={`w-full flex items-center gap-3 px-3 py-1.5 text-left transition-colors border-l-2 ${
                      i === selectedIndex
                        ? "bg-indigo-smoke-900 border-indigo-smoke-500"
                        : "border-transparent hover:bg-slate-grey-900"
                    }`}
                  >
                    <span className="font-mono text-xs text-parchment-200 truncate flex-1 min-w-0">
                      {m.id}
                    </span>
                    <span className="flex-shrink-0 font-mono text-[10px] text-slate-grey-600 tabular-nums w-12 text-right">
                      {formatDownloads(m.downloads)}↓
                    </span>
                  </button>
                ))
              )}

              {/* Footer hints */}
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
                  select
                </span>
                <kbd className="ml-auto px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-500">
                  esc
                </kbd>
                <span className="text-[10px] font-mono text-slate-grey-600">
                  close
                </span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
