import { useHardware } from "./useHardware";

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
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-md bg-slate-grey-950 border border-slate-grey-800 px-3 py-2.5 flex flex-col gap-1.5">
      <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-slate-grey-500">
        {title}
      </span>
      <div className="flex flex-col gap-1">{children}</div>
    </div>
  );
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
            className="h-14 rounded-md bg-slate-grey-900 border border-slate-grey-800"
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
      <MetricBlock title="CPU">
        <StatRow label="name" value={cpu.name} />
        <StatRow label="cores" value={String(cpu.cores)} />
      </MetricBlock>

      {/* GPU */}
      {gpu ? (
        <MetricBlock title="GPU">
          <StatRow label="name" value={gpu.name} />
          {gpu.vram_gb != null && (
            <StatRow label="vram" value={`${gpu.vram_gb.toFixed(1)} GB`} />
          )}
        </MetricBlock>
      ) : (
        <MetricBlock title="GPU">
          <span className="font-body text-xs text-slate-grey-600 italic">
            Not detected.
          </span>
        </MetricBlock>
      )}

      {/* RAM */}
      <MetricBlock title="RAM">
        <StatRow label="total" value={`${memory.total_gb.toFixed(1)} GB`} />
      </MetricBlock>
    </div>
  );
}
