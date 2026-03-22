import { Send, Square } from "lucide-react";

interface InputBarProps {
  input: string;
  canSend: boolean;
  isLoading: boolean;
  onInputChange: (value: string) => void;
  onSend: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
}

export function InputBar({
  input,
  canSend,
  isLoading,
  onInputChange,
  onSend,
  onStop,
}: InputBarProps) {
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter") {
      if (e.shiftKey) return;
      e.preventDefault();
      if (canSend) onSend();
    }
  };

  return (
    <div className="shrink-0 px-4.5 py-2 bg-slate-grey-900 border-t border-slate-grey-800">
      <div className="flex gap-2 items-center bg-slate-grey-950 border border-slate-grey-800 rounded-lg p-2">
        <textarea
          value={input}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={isLoading ? "Generating…" : "Message…"}
          disabled={isLoading}
          rows={1}
          className="flex-1 bg-transparent border-none outline-none resize-none font-body text-sm leading-relaxed min-h-5 text-parchment-200 placeholder:text-slate-grey-500 py-1 disabled:opacity-40"
        />

        {isLoading ? (
          <button
            onClick={onStop}
            className="flex items-center gap-1.5 rounded-md px-3 py-2 font-display text-xs border transition-all duration-200 bg-brick-red-900/40 border-brick-red-800 text-brick-red-400 cursor-pointer hover:bg-brick-red-800/60 hover:border-brick-red-700"
          >
            <Square size={11} fill="currentColor" />
            stop
          </button>
        ) : (
          <button
            onClick={onSend}
            disabled={!canSend}
            className={`flex items-center gap-1.5 rounded-md px-3 py-2 font-display text-xs border transition-all duration-200 ${
              canSend
                ? "bg-indigo-smoke-700 border-indigo-smoke-600 text-parchment-100 cursor-pointer hover:bg-indigo-smoke-600 hover:border-indigo-smoke-500"
                : "bg-transparent border-slate-grey-800 text-slate-grey-600 cursor-default"
            }`}
          >
            <Send size={13} />
            send
          </button>
        )}
      </div>
    </div>
  );
}
