import { Cpu, Zap } from "lucide-react";

interface BackendBadgeProps {
  backend: string;
}

export function BackendBadge({ backend }: BackendBadgeProps) {
  const isVulkan = backend === "vulkan";
  return (
    <span
      className={`inline-flex items-center gap-1 font-mono text-[9px] px-1.5 py-0.5 rounded-full border ${
        isVulkan
          ? "bg-emerald-950/40 border-emerald-800/50 text-emerald-400"
          : "bg-slate-grey-800/60 border-slate-grey-700 text-slate-grey-400"
      }`}
      title={isVulkan ? "Running on GPU via Vulkan" : "Running on CPU"}
    >
      {isVulkan ? <Zap size={8} /> : <Cpu size={8} />}
      {backend}
    </span>
  );
}
