import { useEffect, useRef } from "react";
import type { ChatMessage } from "../../types";
import { Message } from "./Message";

interface MessageListProps {
  messages: ChatMessage[];
  isLoading: boolean;
}

export function MessageList({ messages, isLoading }: MessageListProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="flex-1 overflow-y-auto px-6 pt-2.5">
      {messages.length === 0 ? (
        <div className="h-full flex items-center justify-center">
          <p className="font-body text-sm text-center text-warm-grey-600">
            no messages yet
          </p>
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
