import type { CreativityKey } from "../../types";
import { CREATIVITY_MODES } from "../../constants";
import { SectionHead } from "../ui";
import { HardwarePanel } from "../hardware";

interface SettingsPanelProps {
  model: string;
  availableModels: string[];
  creativity: CreativityKey;
  onModelChange: (model: string) => void;
  onCreativityChange: (key: CreativityKey) => void;
}

export function SettingsPanel({
  model,
  creativity,
  onCreativityChange,
}: SettingsPanelProps) {
  const modelLoaded = model.length > 0;

  return (
    <aside className="w-60 shrink-0 overflow-y-auto px-4 py-3.5 bg-slate-grey-900 border-l border-slate-grey-800 flex flex-col gap-5">
      {/* Model status */}
      <div>
        <SectionHead label="model" />
        <div className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2.5">
          <div className="flex items-center gap-2 mb-1.5">
            <div
              className={`w-1.5 h-1.5 rounded-full flex-shrink-0 transition-all duration-300 ${
                modelLoaded
                  ? "bg-moss-green-500 shadow-[0_0_5px_rgba(115,155,115,0.5)]"
                  : "bg-slate-grey-700"
              }`}
            />
            <span
              className={`font-display text-[11px] uppercase tracking-wide ${
                modelLoaded ? "text-moss-green-600" : "text-slate-grey-600"
              }`}
            >
              {modelLoaded ? "loaded" : "no model"}
            </span>
          </div>
          {modelLoaded ? (
            <p className="font-mono text-xs text-parchment-200 break-all leading-relaxed">
              {model}
            </p>
          ) : (
            <p className="font-body text-xs text-slate-grey-600 italic">
              Search for a model in the header bar to get started.
            </p>
          )}
        </div>
      </div>

      {/* Creativity */}
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
                    className={`font-body text-sm ${
                      isActive
                        ? "font-semibold text-indigo-smoke-400"
                        : "font-normal text-parchment-300"
                    }`}
                  >
                    {label}
                  </div>
                  <div
                    className={`font-body text-xs mt-0.5 ${
                      isActive ? "text-indigo-smoke-500" : "text-slate-grey-500"
                    }`}
                  >
                    {desc}
                  </div>
                </div>
                <div
                  className={`font-mono text-xs shrink-0 ml-2 ${
                    isActive ? "text-indigo-smoke-400" : "text-slate-grey-500"
                  }`}
                >
                  {temp}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Hardware */}
      <div>
        <SectionHead label="hardware" />
        <HardwarePanel />
      </div>
    </aside>
  );
}
