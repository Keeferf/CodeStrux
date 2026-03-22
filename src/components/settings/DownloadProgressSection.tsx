import { X } from "lucide-react";
import { SectionHead } from "../ui";
import { formatBytes, type DownloadProgress } from "../../lib/Download";

interface DownloadProgressSectionProps {
  activeDownload: DownloadProgress;
  onCancel: () => void;
}

export function DownloadProgressSection({
  activeDownload,
  onCancel,
}: DownloadProgressSectionProps) {
  return (
    <div>
      <SectionHead label="downloading" />
      <div className="rounded-md bg-slate-grey-950 border border-indigo-smoke-800 px-3 py-2.5 flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <span className="font-mono text-[10px] text-parchment-300 truncate flex-1 min-w-0">
            {activeDownload.filename}
          </span>
          <button
            onClick={onCancel}
            className="shrink-0 text-slate-grey-600 hover:text-brick-red-400 transition-colors"
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
  );
}
