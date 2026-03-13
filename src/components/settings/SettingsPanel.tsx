import { Trash2, HardDrive, X } from "lucide-react";
import type { CreativityKey } from "../../types";
import { CREATIVITY_MODES } from "../../constants";
import { SectionHead } from "../ui";
import { HardwarePanel } from "../hardware";
import { deleteDownloadedModel, formatBytes } from "../../lib/Download";
import type { DownloadedModel, DownloadProgress } from "../../lib/Download";

interface SettingsPanelProps {
  creativity: CreativityKey;
  downloadedModels: DownloadedModel[];
  activeDownload: DownloadProgress | null;
  onCreativityChange: (key: CreativityKey) => void;
  onCancelDownload: () => void;
  onModelsChanged: () => void;
}

export function SettingsPanel({
  creativity,
  downloadedModels,
  activeDownload,
  onCreativityChange,
  onCancelDownload,
  onModelsChanged,
}: SettingsPanelProps) {
  const handleDeleteModel = async (modelId: string, filename: string) => {
    try {
      await deleteDownloadedModel(modelId, filename);
      onModelsChanged();
    } catch {
      /* ignore */
    }
  };

  return (
    <aside className="w-60 shrink-0 overflow-y-auto px-4 py-3.5 bg-slate-grey-900 border-l border-slate-grey-800 flex flex-col gap-5">
      {/* ── Active download progress ──────────────────────────────────────── */}
      {activeDownload && (
        <div>
          <SectionHead label="downloading" />
          <div className="rounded-md bg-slate-grey-950 border border-indigo-smoke-800 px-3 py-2.5 flex flex-col gap-2">
            <div className="flex items-center justify-between gap-2">
              <span className="font-mono text-[10px] text-parchment-300 truncate flex-1 min-w-0">
                {activeDownload.filename}
              </span>
              <button
                onClick={onCancelDownload}
                className="flex-shrink-0 text-slate-grey-600 hover:text-brick-red-400 transition-colors"
                title="Cancel download"
              >
                <X size={11} />
              </button>
            </div>

            <div className="w-full h-1 bg-slate-grey-800 rounded-full overflow-hidden">
              <div
                className="h-full bg-indigo-smoke-500 rounded-full transition-[width] duration-75"
                style={{ width: `${activeDownload.percent.toFixed(1)}%` }}
              />
            </div>

            <div className="flex items-center justify-between">
              <span className="font-mono text-[10px] text-slate-grey-500 tabular-nums">
                {formatBytes(activeDownload.downloaded)}
                {activeDownload.total > 0 &&
                  ` / ${formatBytes(activeDownload.total)}`}
              </span>
              <span className="font-mono text-[10px] text-indigo-smoke-400 tabular-nums">
                {activeDownload.percent.toFixed(1)}%
              </span>
            </div>

            <p className="font-mono text-[10px] text-slate-grey-600 truncate">
              {activeDownload.model_id}
            </p>
          </div>
        </div>
      )}

      {/* ── Downloaded models ─────────────────────────────────────────────── */}
      <div>
        <SectionHead label="local models" />

        {downloadedModels.length === 0 ? (
          <div className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2.5">
            <p className="font-body text-xs text-slate-grey-600 italic">
              No models downloaded yet. Search and download one from the header.
            </p>
          </div>
        ) : (
          <div className="flex flex-col gap-1.5">
            {downloadedModels.map((m) => (
              <div
                key={`${m.model_id}::${m.filename}`}
                className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2 flex flex-col gap-1 group hover:border-slate-grey-700 transition-colors"
              >
                <div className="flex items-start justify-between gap-1.5">
                  <span className="font-mono text-xs text-parchment-200 break-all leading-tight flex-1">
                    {m.filename}
                  </span>
                  <button
                    onClick={() => handleDeleteModel(m.model_id, m.filename)}
                    className="flex-shrink-0 mt-0.5 text-slate-grey-700 hover:text-brick-red-400 transition-colors opacity-0 group-hover:opacity-100"
                    title="Delete model"
                  >
                    <Trash2 size={11} />
                  </button>
                </div>
                <div className="flex items-center gap-1.5">
                  <HardDrive
                    size={9}
                    className="text-slate-grey-600 flex-shrink-0"
                  />
                  <span className="font-mono text-[10px] text-slate-grey-500 tabular-nums">
                    {formatBytes(m.size)}
                  </span>
                  <span className="font-mono text-[10px] text-slate-grey-700 truncate flex-1 min-w-0">
                    · {m.model_id}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* ── Creativity ────────────────────────────────────────────────────── */}
      <div>
        <SectionHead label="creativity" />
        <div className="flex flex-col gap-1.5">
          {(
            Object.entries(CREATIVITY_MODES) as [
              CreativityKey,
              (typeof CREATIVITY_MODES)[CreativityKey],
            ][]
          ).map(([key, { label, temp, desc }]) => {
            const isActive = creativity === key;
            return (
              <button
                key={key}
                onClick={() => onCreativityChange(key)}
                className={`flex items-center justify-between px-3 py-2.25 rounded-[7px] cursor-pointer transition-all duration-150 text-left border ${
                  isActive
                    ? "bg-indigo-smoke-900/20 border-indigo-smoke-700"
                    : "bg-slate-grey-950 border-slate-grey-800 hover:border-slate-grey-700"
                }`}
              >
                <div>
                  <div
                    className={`font-body text-sm ${isActive ? "font-semibold text-indigo-smoke-400" : "font-normal text-parchment-300"}`}
                  >
                    {label}
                  </div>
                  <div
                    className={`font-body text-xs mt-0.5 ${isActive ? "text-indigo-smoke-500" : "text-slate-grey-500"}`}
                  >
                    {desc}
                  </div>
                </div>
                <div
                  className={`font-mono text-xs shrink-0 ml-2 ${isActive ? "text-indigo-smoke-400" : "text-slate-grey-500"}`}
                >
                  {temp}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* ── Hardware ──────────────────────────────────────────────────────── */}
      <div>
        <SectionHead label="hardware" />
        <HardwarePanel />
      </div>
    </aside>
  );
}
