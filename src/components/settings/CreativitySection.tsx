import { SectionHead } from "../ui";
import { CREATIVITY_MODES } from "../../constants";
import type { CreativityKey } from "../../types";

interface CreativitySectionProps {
  creativity: CreativityKey;
  onChange: (key: CreativityKey) => void;
}

export function CreativitySection({
  creativity,
  onChange,
}: CreativitySectionProps) {
  return (
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
              onClick={() => onChange(key)}
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
  );
}
