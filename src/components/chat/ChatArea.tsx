import type { ChatMessage, CreativityKey, Session } from "../../types";
import { MessageList } from "./MessageList";
import { InputBar } from "./InputBar";

interface ChatAreaProps {
  activeSession: Session;
  messages: ChatMessage[];
  input: string;
  creativity: CreativityKey;
  onInputChange: (value: string) => void;
  onSend: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  isLoading: boolean;
  error: string | null;
}

export function ChatArea({
  activeSession,
  messages,
  input,
  onInputChange,
  onSend,
  onStop,
  isLoading,
  error,
}: ChatAreaProps) {
  const canSend = input.trim().length > 0 && !isLoading;
  const nonSystemCount = messages.filter((m) => m.role !== "system").length;

  return (
    <main className="flex-1 flex flex-col overflow-hidden min-w-0">
      <div className="shrink-0 flex items-center gap-2.5 px-4.5 py-2.5 bg-slate-grey-900 border-b border-slate-grey-800">
        <span className="font-body text-sm font-semibold text-parchment-200">
          {activeSession.title}
        </span>
        <span className="text-[10px] text-slate-grey-500">·</span>
        <span className="font-body text-xs text-slate-grey-500">
          {nonSystemCount} messages
        </span>
      </div>

      <MessageList messages={messages} isLoading={isLoading} />

      {error && (
        <div className="shrink-0 mx-4.5 mb-2 px-3 py-2 rounded-md bg-brick-red-950/60 border border-brick-red-800 font-mono text-xs text-brick-red-400">
          {error}
        </div>
      )}

      <InputBar
        input={input}
        canSend={canSend}
        isLoading={isLoading}
        onInputChange={onInputChange}
        onSend={onSend}
        onStop={onStop}
      />
    </main>
  );
}
