interface SectionHeadProps {
  label: string;
}

export function SectionHead({ label }: SectionHeadProps) {
  return (
    <div className="font-display text-[11px] font-semibold uppercase tracking-[1.5px] mb-2.5 mt-1 pb-1.5 border-b border-slate-grey-800 text-slate-grey-500">
      {label}
    </div>
  );
}
