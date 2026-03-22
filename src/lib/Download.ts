import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface DownloadedModel {
  model_id: string;
  filename: string;
  path: string;
  size: number;
}

export interface DownloadProgress {
  model_id: string;
  filename: string;
  downloaded: number;
  total: number;
  percent: number;
}

export interface DownloadDone {
  model_id: string;
  filename: string;
  path: string;
}

export interface HFFile {
  rfilename: string;
  size?: number;
}

// ── Commands ──────────────────────────────────────────────────────────────────

export function startDownload(
  modelId: string,
  filename: string,
): Promise<void> {
  return invoke("start_download", { modelId, filename });
}

export function cancelDownload(): Promise<void> {
  return invoke("cancel_download");
}

export function getDownloadedModels(): Promise<DownloadedModel[]> {
  return invoke("get_downloaded_models");
}

export function deleteDownloadedModel(
  modelId: string,
  filename: string,
): Promise<void> {
  return invoke("delete_downloaded_model", { modelId, filename });
}

// ── Event listeners ───────────────────────────────────────────────────────────

export function onDownloadProgress(
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download-progress", (e) => cb(e.payload));
}

export function onDownloadDone(
  cb: (d: DownloadDone) => void,
): Promise<UnlistenFn> {
  return listen<DownloadDone>("download-done", (e) => cb(e.payload));
}

export function onDownloadCancelled(cb: () => void): Promise<UnlistenFn> {
  return listen("download-cancelled", () => cb());
}

export function onDownloadError(
  cb: (msg: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("download-error", (e) => cb(e.payload));
}

// ── HuggingFace helpers ───────────────────────────────────────────────────────

/** Fetch GGUF files available for a given model repo. */
export async function fetchGgufFiles(modelId: string): Promise<HFFile[]> {
  const res = await fetch(`https://huggingface.co/api/models/${modelId}`);
  if (!res.ok) throw new Error(`HF API error: ${res.status}`);
  const data = await res.json();
  const siblings: HFFile[] = data.siblings ?? [];
  return siblings.filter((f) => f.rfilename.endsWith(".gguf"));
}

/** Format bytes into a human-readable string. */
export function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`;
  if (bytes >= 1_024) return `${(bytes / 1_024).toFixed(0)} KB`;
  return `${bytes} B`;
}
