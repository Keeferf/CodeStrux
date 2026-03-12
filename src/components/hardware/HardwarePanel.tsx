import { useHardware } from "./useHardware";
import type { GpuInfo } from "./useHardware";

// ── Usage bar ────────────────────────────────────────────────────────────────

function UsageBar({ value }: { value: number }) {
  const clamped = Math.min(100, Math.max(0, value));
  const color =
    clamped > 85
      ? "bg-brick-red-600"
      : clamped > 60
        ? "bg-amber-dust-600"
        : "bg-moss-green-600";

  return (
    <div className="h-0.5 w-full bg-slate-grey-800 rounded-full overflow-hidden mt-1.5">
      <div
        className={`h-full rounded-full transition-all duration-700 ease-out ${color}`}
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}

// ── Stat row ─────────────────────────────────────────────────────────────────

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-2">
      <span className="font-display text-[10px] uppercase tracking-wider text-slate-grey-600 shrink-0">
        {label}
      </span>
      <span className="font-mono text-[11px] text-parchment-300 truncate text-right">
        {value}
      </span>
    </div>
  );
}

// ── Metric block ─────────────────────────────────────────────────────────────

function MetricBlock({
  title,
  usage,
  note,
  children,
}: {
  title: string;
  usage: number | null;
  note?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2.5 flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-slate-grey-500">
          {title}
        </span>
        {usage !== null ? (
          <span className="font-mono text-[11px] text-parchment-400 tabular-nums">
            {usage.toFixed(1)}%
          </span>
        ) : (
          <span className="font-mono text-[9px] text-slate-grey-700 uppercase tracking-wide">
            {note ?? "n/a"}
          </span>
        )}
      </div>
      {usage !== null && <UsageBar value={usage} />}
      <div className="flex flex-col gap-1 mt-0.5">{children}</div>
    </div>
  );
}

// ── GPU usage label ───────────────────────────────────────────────────────────

function gpuVramLabel(gpu: GpuInfo): string {
  if (gpu.vram_used_gb !== null) {
    return `${gpu.vram_used_gb.toFixed(1)} / ${gpu.vram_total_gb.toFixed(1)} GB`;
  }
  return `${gpu.vram_total_gb.toFixed(1)} GB`;
}

function gpuUsagePct(gpu: GpuInfo): number | null {
  if (gpu.vram_used_gb !== null && gpu.vram_total_gb > 0) {
    return (gpu.vram_used_gb / gpu.vram_total_gb) * 100;
  }
  return null;
}

// ── Main panel ───────────────────────────────────────────────────────────────

export function HardwarePanel() {
  const { data, error, isLoading } = useHardware({ interval: 2000 });

  if (isLoading) {
    return (
      <div className="flex flex-col gap-2 animate-pulse">
        {[...Array(3)].map((_, i) => (
          <div
            key={i}
            className="h-20 rounded-md bg-slate-grey-900 border border-slate-grey-800"
          />
        ))}
      </div>
    );
  }

  if (error || !data) {
    return (
      <p className="font-mono text-[11px] text-brick-red-600 px-1">
        {error ?? "Failed to read hardware info."}
      </p>
    );
  }

  const { cpu, memory, gpu } = data;

  return (
    <div className="flex flex-col gap-1.5">
      {/* CPU */}
      <MetricBlock title="CPU" usage={cpu.usage}>
        <StatRow label="name" value={cpu.name} />
        <StatRow label="cores" value={String(cpu.cores)} />
      </MetricBlock>

      {/* Memory */}
      <MetricBlock title="RAM" usage={memory.usage}>
        <StatRow
          label="used"
          value={`${memory.used_gb.toFixed(1)} / ${memory.total_gb.toFixed(1)} GB`}
        />
      </MetricBlock>

      {/* GPU */}
      {gpu ? (
        <MetricBlock
          title="GPU"
          usage={gpuUsagePct(gpu)}
          note="usage unavailable"
        >
          <StatRow label="name" value={gpu.name} />
          <StatRow label="vram" value={gpuVramLabel(gpu)} />
        </MetricBlock>
      ) : (
        <MetricBlock title="GPU" usage={null} note="not detected">
          <span className="font-body text-xs text-slate-grey-600 italic">
            No GPU detected.
          </span>
        </MetricBlock>
      )}
    </div>
  );
}
