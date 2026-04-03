import { LuLoaderCircle, LuDownload, LuCircleCheck } from "react-icons/lu";
import { formatBytes, type HFFile } from "../../../../lib/Download";

interface FileListPanelProps {
  modelId: string;
  ggufFiles: HFFile[];
  isFetchingFiles: boolean;
  downloading: Set<string>;
  downloadedModelIds: string[];
  onDownload: (modelId: string, filename: string) => void;
  onBack: () => void;
}

export function FileListPanel({
  modelId,
  ggufFiles,
  isFetchingFiles,
  downloading,
  downloadedModelIds,
  onDownload,
  onBack,
}: FileListPanelProps) {
  return (
    <div className="absolute top-full left-0 right-0 bg-slate-grey-950 border border-t-0 border-indigo-smoke-700 rounded-b-md overflow-hidden z-50 shadow-[0_8px_24px_rgba(0,0,0,0.4)]">
      {isFetchingFiles ? (
        <div className="flex items-center gap-2 px-3 py-2.5">
          <LuLoaderCircle
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
            const key = `${modelId}::${f.rfilename}`;
            const isDownloading = downloading.has(key);
            const isDone = downloadedModelIds.includes(modelId);

            return (
              <div
                key={f.rfilename}
                className="flex items-center gap-3 px-3 py-1.5 hover:bg-slate-grey-900 border-l-2 border-transparent hover:border-indigo-smoke-700 transition-colors group"
              >
                <span className="font-mono text-xs text-parchment-300 truncate flex-1 min-w-0">
                  {f.rfilename}
                </span>
                {f.size != null && (
                  <span className="font-mono text-[10px] text-slate-grey-600 shrink-0 tabular-nums">
                    {formatBytes(f.size)}
                  </span>
                )}
                <button
                  onClick={() => onDownload(modelId, f.rfilename)}
                  disabled={isDownloading}
                  className={`shrink-0 flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-mono transition-all border ${
                    isDone
                      ? "border-moss-green-800 text-moss-green-600 bg-moss-green-950/40 cursor-default"
                      : isDownloading
                        ? "border-indigo-smoke-800 text-indigo-smoke-500 bg-indigo-smoke-950/40 cursor-wait"
                        : "border-slate-grey-700 text-slate-grey-400 hover:border-indigo-smoke-600 hover:text-indigo-smoke-400 hover:bg-indigo-smoke-950/20"
                  }`}
                >
                  {isDone ? (
                    <>
                      <LuCircleCheck size={9} /> done
                    </>
                  ) : isDownloading ? (
                    <>
                      <LuLoaderCircle size={9} className="animate-spin" /> …
                    </>
                  ) : (
                    <>
                      <LuDownload size={9} /> get
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
          onClick={onBack}
          className="ml-auto px-1 py-px bg-slate-grey-800 border border-slate-grey-700 rounded text-[9px] font-mono text-slate-grey-500 cursor-pointer hover:border-slate-grey-600"
        >
          ← back
        </kbd>
      </div>
    </div>
  );
}
