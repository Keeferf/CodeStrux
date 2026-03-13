import { useState, useRef, useEffect, useCallback } from "react";
import {
  Search,
  Loader2,
  X,
  ChevronLeft,
  Download,
  CheckCircle2,
} from "lucide-react";
import {
  fetchGgufFiles,
  startDownload,
  formatBytes,
  type HFFile,
} from "./lib/Download";

// ── Types ─────────────────────────────────────────────────────────────────────

interface HFModel {
  id: string;
  likes: number;
  downloads: number;
  pipeline_tag?: string;
}

interface ModelSearchProps {
  /** IDs of models that have already been downloaded (for checkmark display). */
  downloadedModelIds: string[];
  /** Called when a download is triggered so App can begin tracking progress. */
  onDownloadStart: (modelId: string, filename: string) => void;
}

// ── Constants ─────────────────────────────────────────────────────────────────

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

async function searchModels(query: string): Promise<HFModel[]> {
  const codingQuery = `${query} code`;
  const res = await fetch(
    `https://huggingface.co/api/models?search=${encodeURIComponent(codingQuery)}&pipeline_tag=text-generation&limit=20&sort=downloads&direction=-1`,
  );
  const all: HFModel[] = await res.json();
  const coderOnly = all.filter((m) => isCoderModel(m.id));
  return (coderOnly.length > 0 ? coderOnly : all).slice(0, 8);
}

// ── Component ─────────────────────────────────────────────────────────────────

export function ModelSearch({
  downloadedModelIds,
  onDownloadStart,
}: ModelSearchProps) {
  const [isOpen, setIsOpen] = useState(false);

  // Step 1: model search
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<HFModel[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);

  // Step 2: GGUF file list for a chosen model
  const [expandedModel, setExpandedModel] = useState<HFModel | null>(null);
  const [ggufFiles, setGgufFiles] = useState<HFFile[]>([]);
  const [isFetchingFiles, setIsFetchingFiles] = useState(false);

  // Track which files are currently downloading in this session
  const [downloading, setDownloading] = useState<Set<string>>(new Set());

  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── Search logic ──────────────────────────────────────────────────────────

  const search = useCallback(async (q: string) => {
    if (!q.trim()) {
      setResults([]);
      return;
    }
    setIsSearching(true);
    try {
      setResults(await searchModels(q));
    } catch {
      setResults([]);
    } finally {
      setIsSearching(false);
    }
  }, []);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => search(query), 300);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [query, search]);

  // ── Open / close ──────────────────────────────────────────────────────────

  const open = () => {
    setIsOpen(true);
    setQuery("");
    setResults([]);
    setSelectedIndex(-1);
    setExpandedModel(null);
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  const close = () => {
    setIsOpen(false);
    setQuery("");
    setResults([]);
    setSelectedIndex(-1);
    setExpandedModel(null);
    setGgufFiles([]);
  };

  // ── Outside click ─────────────────────────────────────────────────────────

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

  // ── Step 2: expand a model to show its GGUF files ─────────────────────────

  const expandModel = async (m: HFModel) => {
    setExpandedModel(m);
    setGgufFiles([]);
    setIsFetchingFiles(true);
    try {
      const files = await fetchGgufFiles(m.id);
      setGgufFiles(files);
    } catch {
      setGgufFiles([]);
    } finally {
      setIsFetchingFiles(false);
    }
  };

  const backToSearch = () => {
    setExpandedModel(null);
    setGgufFiles([]);
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  // ── Download ──────────────────────────────────────────────────────────────

  const handleDownload = async (modelId: string, filename: string) => {
    const key = `${modelId}::${filename}`;
    if (downloading.has(key)) return;
    setDownloading((prev) => new Set(prev).add(key));
    onDownloadStart(modelId, filename);
    try {
      await startDownload(modelId, filename);
    } finally {
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  // ── Keyboard navigation (step 1 only) ────────────────────────────────────

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      close();
      return;
    }
    if (expandedModel) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, -1));
    } else if (e.key === "Enter" && selectedIndex >= 0) {
      expandModel(results[selectedIndex]);
    }
  };

  const showDropdown =
    isOpen && !expandedModel && (results.length > 0 || (query && !isSearching));
  const showFilePanel = isOpen && expandedModel !== null;

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div
      ref={containerRef}
      className="relative flex-1 max-w-[480px]"
      style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
    >
      {/* ── Closed trigger bar ── */}
      {!isOpen ? (
        <button
          onClick={open}
          className="w-full flex items-center gap-2 h-6 bg-slate-grey-950 border border-slate-grey-800 rounded-md px-3 hover:border-slate-grey-600 hover:bg-slate-grey-900 transition-all duration-150 group"
        >
          <Download
            size={11}
            className="text-slate-grey-600 group-hover:text-indigo-smoke-500 flex-shrink-0 transition-colors"
          />
          <span className="font-mono text-xs flex-1 text-left truncate text-slate-grey-500 group-hover:text-parchment-400 transition-colors">
            Download a coder model…
          </span>
          {downloadedModelIds.length > 0 && (
            <span className="font-mono text-[10px] text-moss-green-600 flex-shrink-0">
              {downloadedModelIds.length} downloaded
            </span>
          )}
        </button>
      ) : (
        /* ── Open state ── */
        <div className="w-full">
          {/* Search input (always visible when open) */}
          <div className="flex items-center gap-2 h-6 bg-slate-grey-950 border border-indigo-smoke-700 rounded-t-md px-3 shadow-[0_0_0_1px_rgba(114,139,180,0.15)]">
            {expandedModel ? (
              <button
                onClick={backToSearch}
                className="text-indigo-smoke-400 hover:text-parchment-300 transition-colors flex-shrink-0"
                title="Back to search"
              >
                <ChevronLeft size={12} />
              </button>
            ) : isSearching ? (
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

            {expandedModel ? (
              <span className="flex-1 font-mono text-xs text-parchment-300 truncate">
                {expandedModel.id}
              </span>
            ) : (
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
            )}

            {!expandedModel && query && (
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

            <button
              onClick={close}
              className="text-slate-grey-700 hover:text-slate-grey-400 transition-colors flex-shrink-0"
            >
              <X size={11} />
            </button>
          </div>

          {/* ── Step 1: Search results dropdown ── */}
          {showDropdown && (
            <div className="absolute top-full left-0 right-0 bg-slate-grey-950 border border-t-0 border-indigo-smoke-700 rounded-b-md overflow-hidden z-50 shadow-[0_8px_24px_rgba(0,0,0,0.4)]">
              {results.length === 0 ? (
                <div className="px-3 py-2 text-xs font-mono text-slate-grey-600">
                  No coder models found for "{query}"
                </div>
              ) : (
                results.map((m, i) => {
                  const alreadyDownloaded = downloadedModelIds.includes(m.id);
                  return (
                    <button
                      key={m.id}
                      onClick={() => expandModel(m)}
                      onMouseEnter={() => setSelectedIndex(i)}
                      className={`w-full flex items-center gap-3 px-3 py-1.5 text-left transition-colors border-l-2 ${
                        i === selectedIndex
                          ? "bg-indigo-smoke-900 border-indigo-smoke-500"
                          : "border-transparent hover:bg-slate-grey-900"
                      }`}
                    >
                      {alreadyDownloaded && (
                        <CheckCircle2
                          size={11}
                          className="text-moss-green-500 flex-shrink-0"
                        />
                      )}
                      <span className="font-mono text-xs text-parchment-200 truncate flex-1 min-w-0">
                        {m.id}
                      </span>
                      <span className="flex-shrink-0 font-mono text-[10px] text-slate-grey-600 tabular-nums w-12 text-right">
                        {formatDownloads(m.downloads)}↓
                      </span>
                      <ChevronLeft
                        size={10}
                        className="text-slate-grey-700 rotate-180 flex-shrink-0"
                      />
                    </button>
                  );
                })
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
                  expand
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

          {/* ── Step 2: GGUF file list ── */}
          {showFilePanel && (
            <div className="absolute top-full left-0 right-0 bg-slate-grey-950 border border-t-0 border-indigo-smoke-700 rounded-b-md overflow-hidden z-50 shadow-[0_8px_24px_rgba(0,0,0,0.4)]">
              {isFetchingFiles ? (
                <div className="flex items-center gap-2 px-3 py-2.5">
                  <Loader2
                    size={11}
                    className="text-indigo-smoke-400 animate-spin"
                  />
                  <span className="font-mono text-xs text-slate-grey-500">
                    Fetching GGUF files…
                  </span>
                </div>
              ) : ggufFiles.length === 0 ? (
                <div className="px-3 py-2 text-xs font-mono text-slate-grey-600">
                  No GGUF files found for this model.
                </div>
              ) : (
                <div className="max-h-52 overflow-y-auto">
                  {ggufFiles.map((f) => {
                    const key = `${expandedModel!.id}::${f.rfilename}`;
                    const isDownloading = downloading.has(key);
                    const isDone = downloadedModelIds.includes(
                      expandedModel!.id,
                    );

                    return (
                      <div
                        key={f.rfilename}
                        className="flex items-center gap-3 px-3 py-1.5 hover:bg-slate-grey-900 border-l-2 border-transparent hover:border-indigo-smoke-700 transition-colors group"
                      >
                        <span className="font-mono text-xs text-parchment-300 truncate flex-1 min-w-0">
                          {f.rfilename}
                        </span>
                        {f.size != null && (
                          <span className="font-mono text-[10px] text-slate-grey-600 flex-shrink-0 tabular-nums">
                            {formatBytes(f.size)}
                          </span>
                        )}
                        <button
                          onClick={() =>
                            handleDownload(expandedModel!.id, f.rfilename)
                          }
                          disabled={isDownloading}
                          className={`flex-shrink-0 flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-mono transition-all border ${
                            isDone
                              ? "border-moss-green-800 text-moss-green-600 bg-moss-green-950/40 cursor-default"
                              : isDownloading
                                ? "border-indigo-smoke-800 text-indigo-smoke-500 bg-indigo-smoke-950/40 cursor-wait"
                                : "border-slate-grey-700 text-slate-grey-400 hover:border-indigo-smoke-600 hover:text-indigo-smoke-400 hover:bg-indigo-smoke-950/20"
                          }`}
                        >
                          {isDone ? (
                            <>
                              <CheckCircle2 size={9} /> done
                            </>
                          ) : isDownloading ? (
                            <>
                              <Loader2 size={9} className="animate-spin" /> …
                            </>
                          ) : (
                            <>
                              <Download size={9} /> get
                            </>
                          )}
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}

              <div className="flex items-center gap-2 px-3 py-1 border-t border-slate-grey-800">
                <span className="text-[10px] font-mono text-slate-grey-600">
                  GGUF files · progress shown in settings
                </span>
                <kbd
                  onClick={backToSearch}
                  className="ml-auto px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-500 cursor-pointer hover:border-slate-grey-600"
                >
                  ← back
                </kbd>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
