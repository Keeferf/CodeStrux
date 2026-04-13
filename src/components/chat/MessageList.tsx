import { useEffect, useRef } from "react";
import type { ChatMessage } from "../../types";
import { Message } from "./Message";

interface MessageListProps {
  messages: ChatMessage[];
  isLoading: boolean;
  onScrollbarWidth?: (width: number) => void;
}

export function MessageList({
  messages,
  isLoading,
  onScrollbarWidth,
}: MessageListProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    if (!containerRef.current || !onScrollbarWidth) return;
    const el = containerRef.current;
    const measure = () => onScrollbarWidth(el.offsetWidth - el.clientWidth);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [onScrollbarWidth]);

  return (
    <div
      ref={containerRef}
      className="flex-1 overflow-y-auto px-40 pt-2.5 pb-24 flex flex-col"
      style={{ scrollbarGutter: "stable" }}
    >
      {messages.length === 0 ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <div className="font-display text-sm uppercase tracking-wider text-warm-grey-600 mb-2">
              No messages yet
            </div>
            <p className="font-display text-xs text-warm-grey-600">
              Type a message below to start the conversation
            </p>
          </div>
        </div>
      ) : (
        <>
          {messages.map((m) => (
            <Message key={m.id} message={m} />
          ))}
          <div className="pt-2.5 pb-5">
            <div className="inline-flex items-center gap-1.5 font-body text-xs text-warm-grey-600">
              {isLoading ? (
                <>
                  <span className="inline-flex gap-0.75">
                    <span className="w-1.25 h-1.25 rounded-full bg-moss-green-500 animate-pulse [animation-delay:0s]" />
                    <span className="w-1.25 h-1.25 rounded-full bg-moss-green-500 animate-pulse [animation-delay:0.2s]" />
                    <span className="w-1.25 h-1.25 rounded-full bg-moss-green-500 animate-pulse [animation-delay:0.4s]" />
                  </span>
                  generating…
                </>
              ) : (
                <>
                  <span className="inline-flex gap-0.75">
                    <span className="w-1.25 h-1.25 rounded-full bg-indigo-smoke-500 animate-pulse [animation-delay:0s]" />
                    <span className="w-1.25 h-1.25 rounded-full bg-indigo-smoke-500 animate-pulse [animation-delay:0.2s]" />
                    <span className="w-1.25 h-1.25 rounded-full bg-indigo-smoke-500 animate-pulse [animation-delay:0.4s]" />
                  </span>
                  awaiting input
                </>
              )}
            </div>
          </div>
        </>
      )}
      <div ref={bottomRef} />
    </div>
  );
}
