import { useState, useRef, useEffect, useCallback } from "react";
import {
  fetchGgufFiles,
  startDownload,
  type HFFile,
} from "../../../../lib/Download";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface HFModel {
  id: string;
  likes: number;
  downloads: number;
  pipeline_tag?: string;
}

// ── API helpers ───────────────────────────────────────────────────────────────

export function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

async function searchModels(query: string): Promise<HFModel[]> {
  const res = await fetch(
    `https://huggingface.co/api/models?search=${encodeURIComponent(query)}&pipeline_tag=text-generation&filter=gguf&limit=20&sort=downloads&direction=-1`,
  );
  const all: HFModel[] = await res.json();
  return all.slice(0, 8);
}

// ── Hook ──────────────────────────────────────────────────────────────────────

export function useModelSearch(
  onDownloadStart: (modelId: string, filename: string) => void,
) {
  const [isOpen, setIsOpen] = useState(false);

  // Step 1: search
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<HFModel[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);

  // Step 2: GGUF file list
  const [expandedModel, setExpandedModel] = useState<HFModel | null>(null);
  const [ggufFiles, setGgufFiles] = useState<HFFile[]>([]);
  const [isFetchingFiles, setIsFetchingFiles] = useState(false);

  // Download tracking
  const [downloading, setDownloading] = useState<Set<string>>(new Set());

  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── Debounced search ──────────────────────────────────────────────────────

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

  // ── Expand model → GGUF files ─────────────────────────────────────────────

  const expandModel = async (m: HFModel) => {
    setExpandedModel(m);
    setGgufFiles([]);
    setIsFetchingFiles(true);
    try {
      setGgufFiles(await fetchGgufFiles(m.id));
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

  // ── Keyboard navigation ───────────────────────────────────────────────────

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

  // ── Derived state ─────────────────────────────────────────────────────────

  const showDropdown =
    isOpen &&
    !expandedModel &&
    (results.length > 0 || (!!query && !isSearching));
  const showFilePanel = isOpen && expandedModel !== null;

  return {
    // refs
    inputRef,
    containerRef,
    // open/close
    isOpen,
    open,
    close,
    // step 1
    query,
    setQuery,
    results,
    isSearching,
    selectedIndex,
    setSelectedIndex,
    showDropdown,
    handleKeyDown,
    // step 2
    expandedModel,
    expandModel,
    backToSearch,
    ggufFiles,
    isFetchingFiles,
    showFilePanel,
    // download
    downloading,
    handleDownload,
  };
}
