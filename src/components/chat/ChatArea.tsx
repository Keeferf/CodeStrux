import { useState } from "react";
import type { ChatMessage, CreativityKey, Session } from "../../types";
import { MessageList } from "./MessageList";
import { InputBar } from "./InputBar";
import { AttachedFile } from "./FileAttachment";

interface ChatAreaProps {
  activeSession: Session;
  messages: ChatMessage[];
  input: string;
  creativity: CreativityKey;
  attachedFiles: AttachedFile[];
  onInputChange: (value: string) => void;
  onSend: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onFilesAttach: (files: AttachedFile[]) => void;
  onFileRemove: (fileId: string) => void;
  isLoading: boolean;
  error: string | null;
}

export function ChatArea({
  activeSession,
  messages,
  input,
  attachedFiles,
  onInputChange,
  onSend,
  onStop,
  onFilesAttach,
  onFileRemove,
  isLoading,
  error,
}: ChatAreaProps) {
  const [scrollbarWidth, setScrollbarWidth] = useState(0);

  const canSend =
    (input.trim().length > 0 || attachedFiles.length > 0) && !isLoading;
  const nonSystemCount = messages.filter((m) => m.role !== "system").length;

  return (
    <main className="flex-1 flex flex-col overflow-hidden min-w-0 relative">
      <div className="shrink-0 flex items-center gap-2.5 px-4.5 py-2 bg-slate-grey-900 border-b border-slate-grey-800">
        <span className="font-body text-sm font-semibold text-parchment-200">
          {activeSession.title}
        </span>
        <span className="text-[10px] text-slate-grey-500">·</span>
        <span className="font-body text-xs text-slate-grey-500">
          {nonSystemCount} messages
        </span>
      </div>

      <div className="flex-1 overflow-hidden min-w-0 flex flex-col">
        <MessageList
          messages={messages}
          isLoading={isLoading}
          onScrollbarWidth={setScrollbarWidth}
        />
      </div>

      {error && (
        <div className="mx-4.5 mb-2 px-3 py-2 rounded-md bg-red-950/60 border border-red-900 font-mono text-xs text-red-400">
          {error}
        </div>
      )}

      <InputBar
        input={input}
        canSend={canSend}
        isLoading={isLoading}
        attachedFiles={attachedFiles}
        onInputChange={onInputChange}
        onSend={onSend}
        onStop={onStop}
        onFilesAttach={onFilesAttach}
        onFileRemove={onFileRemove}
        scrollbarWidth={scrollbarWidth}
      />
    </main>
  );
}
