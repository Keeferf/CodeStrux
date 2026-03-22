import type { ChatMessage } from "../../types";
import { CodeBlock } from "./CodeBlock";

interface MessageProps {
  message: ChatMessage;
}

export function Message({ message }: MessageProps) {
  if (message.role === "system") {
    return (
      <div className="text-center pt-1.5 pb-4">
        <span className="font-body text-xs text-warm-grey-500 bg-slate-grey-900 border border-slate-grey-800 rounded-full px-3 py-0.75 tracking-wide">
          {message.content}
        </span>
      </div>
    );
  }

  const isUser = message.role === "user";
  const parts = message.content.split(/(```[\s\S]*?```)/g);

  return (
    <div
      className={`flex gap-3 py-3.5 items-start ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      <div
        className={`w-7.5 h-7.5 rounded-md shrink-0 flex items-center justify-center font-mono text-[11px] font-bold ${
          isUser
            ? "bg-indigo-smoke-900/60 border border-indigo-smoke-700 text-indigo-smoke-400"
            : "bg-slate-grey-900 border border-slate-grey-800 text-warm-grey-500"
        }`}
      >
        {isUser ? "you" : "ai"}
      </div>

      <div className="max-w-[76%] min-w-0">
        <div
          className={`font-display text-[10px] mb-1.5 text-warm-grey-600 ${isUser ? "text-right" : "text-left"}`}
        >
          {isUser ? "user" : "assistant"}
        </div>
        <div
          className={`px-3.75 py-3 font-body text-sm leading-[1.7] text-parchment-200 ${
            isUser
              ? "bg-indigo-smoke-950/50 border border-indigo-smoke-800 rounded-[10px_2px_10px_10px]"
              : "bg-slate-grey-900 border border-slate-grey-800 rounded-[2px_10px_10px_10px]"
          }`}
        >
          {parts.map((part, i) => {
            if (part.startsWith("```")) {
              const lines = part.slice(3, -3).split("\n");
              const lang = lines[0]?.trim() || "code";
              const code = lines.slice(1).join("\n");
              return <CodeBlock key={i} lang={lang} code={code} />;
            }
            return (
              <span key={i} className="whitespace-pre-wrap">
                {part}
              </span>
            );
          })}
        </div>
      </div>
    </div>
  );
}
