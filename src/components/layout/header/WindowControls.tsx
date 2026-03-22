import { Minus, Square, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function WindowControls() {
  const minimize = () => getCurrentWindow().minimize();
  const maximize = () => getCurrentWindow().toggleMaximize();
  const close = () => getCurrentWindow().close();

  return (
    <div
      className="flex items-center self-stretch w-40 justify-end"
      style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
    >
      <button
        onClick={minimize}
        className="flex items-center justify-center w-9 h-full text-slate-grey-400 hover:text-parchment-200 hover:bg-slate-grey-800 transition-colors"
        aria-label="Minimize"
      >
        <Minus size={13} strokeWidth={2} />
      </button>
      <button
        onClick={maximize}
        className="flex items-center justify-center w-9 h-full text-slate-grey-400 hover:text-parchment-200 hover:bg-slate-grey-800 transition-colors"
        aria-label="Maximize"
      >
        <Square size={11} strokeWidth={2} />
      </button>
      <button
        onClick={close}
        className="flex items-center justify-center w-9 h-full text-slate-grey-400 hover:text-parchment-100 hover:bg-red-800 transition-colors"
        aria-label="Close"
      >
        <X size={13} strokeWidth={2} />
      </button>
    </div>
  );
}
