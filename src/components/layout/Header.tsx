import { ChevronRight, Minus, Square, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface HeaderProps {
  model: string;
}

export function Header({ model }: HeaderProps) {
  const modelLoaded = model.length > 0;

  const minimize = () => getCurrentWindow().minimize();
  const maximize = () => getCurrentWindow().toggleMaximize();
  const close = () => getCurrentWindow().close();

  return (
    <header
      className="h-9 shrink-0 flex items-center justify-between bg-slate-grey-900 border-b border-slate-grey-800"
      style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
    >
      {/* Left: Logo + App name */}
      <div
        className="flex items-center gap-2.5 pl-3"
        style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
      >
        <img src="/logo.png" alt="CodeStrux" className="w-5.5 h-5.5" />
        <span className="font-display text-sm font-semibold text-parchment-200 tracking-wide">
          CodeStrux
        </span>
      </div>

      {/* Center: Model status */}
      <div
        className="flex items-center gap-2 bg-slate-grey-950 border border-slate-grey-800 rounded-md px-3 py-1 cursor-pointer hover:border-slate-grey-700 transition-colors"
        style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
      >
        <div
          className={`w-1.75 h-1.75 rounded-full transition-all duration-300 ${
            modelLoaded
              ? "bg-moss-green-500 shadow-[0_0_6px_rgba(115,155,115,0.6)]"
              : "bg-slate-grey-700"
          }`}
        />
        <span
          className={`font-mono text-xs ${modelLoaded ? "text-parchment-300" : "text-slate-grey-500"}`}
        >
          {modelLoaded ? model : "no model loaded"}
        </span>
        <ChevronRight size={13} className="text-slate-grey-500" />
      </div>

      {/* Right: Custom window controls */}
      <div
        className="flex items-center self-stretch"
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
          className="flex items-center justify-center w-9 h-full text-slate-grey-400 hover:text-parchment-100 hover:bg-brick-red-800 transition-colors"
          aria-label="Close"
        >
          <X size={13} strokeWidth={2} />
        </button>
      </div>
    </header>
  );
}
