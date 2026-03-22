import { HardDrive, Play, Trash2 } from "lucide-react";
import { SectionHead } from "../ui";
import { BackendBadge } from "./BackendBadge";
import { formatBytes, type DownloadedModel } from "../../lib/Download";
import type { LoadedModelInfo } from "./useLocalModel";

interface ModelListSectionProps {
  downloadedModels: DownloadedModel[];
  loadedModel: LoadedModelInfo | null;
  isLoading: boolean;
  isLoaded: (modelId: string, filename: string) => boolean;
  onLoad: (modelId: string, filename: string) => void;
  onDelete: (modelId: string, filename: string) => void;
}

export function ModelListSection({
  downloadedModels,
  loadedModel,
  isLoading,
  isLoaded,
  onLoad,
  onDelete,
}: ModelListSectionProps) {
  return (
    <div>
      <SectionHead label="Models" />

      {downloadedModels.length === 0 ? (
        <div className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2.5">
          <p className="font-body text-xs text-slate-grey-600 italic">
            No models downloaded yet. Search and download one from the header.
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-1.5">
          {downloadedModels.map((m) => {
            const active = isLoaded(m.model_id, m.filename);
            return (
              <div
                key={`${m.model_id}::${m.filename}`}
                className={`rounded-md bg-slate-grey-950 border px-3 py-2 flex flex-col gap-1 group transition-colors ${
                  active
                    ? "border-emerald-800/40"
                    : "border-slate-grey-800 hover:border-slate-grey-700"
                }`}
              >
                <div className="flex items-start justify-between gap-1.5">
                  <span className="font-mono text-xs text-parchment-200 break-all leading-tight flex-1">
                    {m.filename}
                  </span>
                  <div className="flex items-center gap-1 shrink-0 mt-0.5">
                    {!active && (
                      <button
                        onClick={() => onLoad(m.model_id, m.filename)}
                        disabled={isLoading}
                        className="text-slate-grey-600 hover:text-emerald-400 transition-colors disabled:opacity-30 disabled:cursor-not-allowed opacity-0 group-hover:opacity-100"
                        title="Load model"
                      >
                        <Play size={11} />
                      </button>
                    )}
                    <button
                      onClick={() => onDelete(m.model_id, m.filename)}
                      disabled={active || isLoading}
                      className="text-slate-grey-700 hover:text-brick-red-400 transition-colors disabled:opacity-30 disabled:cursor-not-allowed opacity-0 group-hover:opacity-100"
                      title={
                        active
                          ? "Unload the model before deleting"
                          : "Delete model"
                      }
                    >
                      <Trash2 size={11} />
                    </button>
                  </div>
                </div>
                <div className="flex items-center gap-1.5">
                  <HardDrive
                    size={9}
                    className="text-slate-grey-600 shrink-0"
                  />
                  <span className="font-mono text-[10px] text-slate-grey-500 tabular-nums">
                    {formatBytes(m.size)}
                  </span>
                  <span className="font-mono text-[10px] text-slate-grey-700 truncate flex-1 min-w-0">
                    · {m.model_id}
                  </span>
                  {active && loadedModel && (
                    <BackendBadge backend={loadedModel.backend} />
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
