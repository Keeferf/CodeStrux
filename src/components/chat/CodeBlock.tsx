import { useState } from "react";

interface CodeBlockProps {
  lang: string;
  code: string;
}

export function CodeBlock({ lang, code }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard?.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="my-2.5 overflow-hidden rounded-md bg-slate-grey-950 border border-slate-grey-800">
      <div className="flex justify-between items-center px-3 py-1.25 bg-slate-grey-900 border-b border-slate-grey-800">
        <span className="font-mono text-[11px] tracking-wider text-warm-grey-600">
          {lang}
        </span>
        <button
          onClick={handleCopy}
          className={`font-mono bg-transparent border rounded-sm px-2 py-0.5 text-[11px] cursor-pointer transition-colors duration-200 ${
            copied
              ? "text-indigo-smoke-400 border-indigo-smoke-700"
              : "text-warm-grey-600 border-slate-grey-800 hover:text-warm-grey-400"
          }`}
        >
          {copied ? "copied!" : "copy"}
        </button>
      </div>
      <pre className="m-0 px-4 py-3.5 overflow-x-auto font-mono text-[13px] leading-[1.65] text-parchment-200">
        <code>{code}</code>
      </pre>
    </div>
  );
}
