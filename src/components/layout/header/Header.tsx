import { WindowControls } from "./WindowControls";
import { ModelSearch } from "../header/model-search/ModelSearch";

interface HeaderProps {
  model: string;
  onModelChange: (model: string) => void;
  downloadedModelIds: string[];
  onDownloadStart: (modelId: string, filename: string) => void;
}

export function Header({ downloadedModelIds, onDownloadStart }: HeaderProps) {
  return (
    <header
      className="h-9 shrink-0 flex items-center justify-between bg-slate-grey-900 border-b border-slate-grey-800"
      style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
    >
      {/* Left: Logo + app name */}
      <div
        className="flex items-center gap-2.5 pl-3 w-58"
        style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
      >
        <img src="/icon.svg" alt="CodeStrux" className="w-5.5 h-5.5" />
        <span className="font-display text-sm font-semibold text-parchment-200 tracking-wide">
          CodeStrux
        </span>
      </div>

      {/* Center: Model search / download */}
      <ModelSearch
        downloadedModelIds={downloadedModelIds}
        onDownloadStart={onDownloadStart}
      />

      {/* Right: Window controls */}
      <WindowControls />
    </header>
  );
}
