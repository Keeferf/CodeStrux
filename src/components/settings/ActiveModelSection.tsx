import { LuLoaderCircle, LuSquare, LuPlay } from "react-icons/lu";
import { SectionHead } from "../ui";
import { BackendBadge } from "./BackendBadge";
import type { LoadedModelInfo, LoadingState } from "./useLocalModel";

interface ActiveModelSectionProps {
  loadedModel: LoadedModelInfo | null;
  loadingState: LoadingState;
  onUnload: () => void;
}

export function ActiveModelSection({
  loadedModel,
  loadingState,
  onUnload,
}: ActiveModelSectionProps) {
  const isLoading = loadingState.status === "loading";

  return (
    <div>
      <SectionHead label="Active Model" />

      {isLoading && (
        <div className="rounded-md bg-slate-grey-950 border border-indigo-smoke-800/50 px-3 py-2.5 flex items-center gap-2">
          <LuLoaderCircle
            size={11}
            className="text-indigo-smoke-400 animate-spin shrink-0"
          />
          <span className="font-mono text-[10px] text-indigo-smoke-400">
            {loadingState.backend
              ? `loading on ${loadingState.backend}…`
              : "loading model…"}
          </span>
        </div>
      )}

      {loadedModel && !isLoading && (
        <div className="rounded-md bg-slate-grey-950 border border-emerald-800/40 px-3 py-2.5 flex flex-col gap-1.5">
          <div className="flex items-start justify-between gap-1.5">
            <span className="font-mono text-xs text-parchment-200 break-all leading-tight flex-1">
              {loadedModel.filename}
            </span>
            <button
              onClick={onUnload}
              className="shrink-0 mt-0.5 text-slate-grey-600 hover:text-red-800 transition-colors"
              title="Unload model"
            >
              <LuSquare size={11} />
            </button>
          </div>
          <div className="flex items-center gap-1.5 flex-wrap">
            <BackendBadge backend={loadedModel.backend} />
            <span className="font-mono text-[10px] text-slate-grey-600 truncate">
              {loadedModel.model_id}
            </span>
          </div>
        </div>
      )}

      {!loadedModel && !isLoading && (
        <div className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2.5">
          <p className="font-body text-xs text-slate-grey-600 italic">
            No model loaded. Click{" "}
            <LuPlay size={10} className="inline-block align-text-center" /> on a
            downloaded model below.
          </p>
        </div>
      )}
    </div>
  );
}
