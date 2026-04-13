import { LuSend, LuSquare, LuPaperclip } from "react-icons/lu";
import { useRef } from "react";
import {
  FileAttachment,
  AttachedFile,
  FileAttachmentRef,
} from "./FileAttachment";

interface InputBarProps {
  input: string;
  canSend: boolean;
  isLoading: boolean;
  attachedFiles: AttachedFile[];
  onInputChange: (value: string) => void;
  onSend: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onFilesAttach: (files: AttachedFile[]) => void;
  onFileRemove: (fileId: string) => void;
  scrollbarWidth: number;
}

export function InputBar({
  input,
  canSend,
  isLoading,
  attachedFiles,
  onInputChange,
  onSend,
  onStop,
  onFilesAttach,
  onFileRemove,
  scrollbarWidth,
}: InputBarProps) {
  const fileAttachRef = useRef<FileAttachmentRef | null>(null);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (canSend && !isLoading) onSend();
    }
  };

  return (
    <div className="absolute bottom-0 left-0 right-0 flex justify-center pb-4 pointer-events-none bg-slate-grey-950">
      <div
        className="pointer-events-auto w-full max-w-[calc(100%-19rem)] min-w-0 flex flex-col gap-0"
        style={{ transform: `translateX(-${scrollbarWidth / 2}px)` }}
      >
        {/* File attachment area */}
        <div
          className={`transition-all duration-200 overflow-hidden ${
            attachedFiles.length > 0
              ? "max-h-96 opacity-100 bg-slate-grey-900 border border-b-0 border-slate-grey-700 rounded-t-xl px-4 pt-3 pb-2"
              : "max-h-0 opacity-0 border-0 p-0"
          }`}
        >
          <FileAttachment
            ref={fileAttachRef}
            onFilesAttach={onFilesAttach}
            onFileRemove={onFileRemove}
            attachedFiles={attachedFiles}
            disabled={isLoading}
          />
        </div>

        {/* Floating input pill */}
        <div
          className={`flex gap-1.5 items-center bg-slate-grey-900 border border-slate-grey-700 p-3 shadow-xl shadow-black/40 ${
            attachedFiles.length > 0
              ? "rounded-b-xl rounded-t-none"
              : "rounded-xl"
          }`}
        >
          {/* Paperclip trigger */}
          <button
            onClick={() => fileAttachRef.current?.openFilePicker()}
            disabled={isLoading}
            className="flex items-center justify-center rounded-md p-2 border transition-all duration-200 text-slate-grey-500 border-slate-grey-700/60 hover:bg-indigo-smoke-700 hover:border-indigo-smoke-600 hover:text-parchment-100 disabled:opacity-40 disabled:cursor-default cursor-pointer"
            aria-label="Attach files"
          >
            <LuPaperclip size={14} />
          </button>

          {/* Text input */}
          <textarea
            value={input}
            onChange={(e) => onInputChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={isLoading ? "Generating…" : "Message…"}
            disabled={isLoading}
            rows={1}
            className="flex-1 bg-transparent border-none outline-none resize-none font-body text-sm text-parchment-200 placeholder:text-slate-grey-500 disabled:opacity-40 py-1.5 leading-tight"
          />

          {/* Actions */}
          <div className="flex gap-2.5 items-center">
            {isLoading ? (
              <button
                onClick={onStop}
                className="flex items-center justify-center rounded-md p-2 border transition-all duration-200 bg-brick-red-900/40 border-brick-red-800 text-brick-red-400 hover:bg-brick-red-800/60 hover:border-brick-red-700 cursor-pointer"
              >
                <LuSquare size={14} fill="currentColor" />
              </button>
            ) : (
              <button
                onClick={onSend}
                disabled={!canSend}
                className={`flex items-center justify-center rounded-md p-2 border transition-all duration-200 ${
                  canSend
                    ? "bg-indigo-smoke-700 border-indigo-smoke-600 text-parchment-100 hover:bg-indigo-smoke-600 hover:border-indigo-smoke-500 cursor-pointer"
                    : "bg-transparent border-slate-grey-700/60 text-slate-grey-600 cursor-default"
                }`}
              >
                <LuSend size={14} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
