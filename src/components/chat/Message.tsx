import type { ChatMessage } from "../../types";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
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
  const parts = !isUser ? message.content.split(/(```[\s\S]*?```)/g) : null;

  return (
    <div
      className={`flex gap-3 py-3.5 items-start ${isUser ? "flex-row-reverse" : "flex-row"}`}
    >
      <div className={`min-w-0 ${isUser ? "max-w-[75%] self-end" : "flex-1"}`}>
        <div
          className={`px-3.75 py-3 font-body text-sm leading-[1.7] text-parchment-200 ${
            isUser
              ? "bg-indigo-smoke-800/60 border border-indigo-smoke-600 rounded-[10px_2px_10px_10px]"
              : "bg-slate-grey-900 border border-slate-grey-800 rounded-[2px_10px_10px_10px]"
          }`}
        >
          {isUser ? (
            <span className="whitespace-pre-wrap">{message.content}</span>
          ) : (
            parts?.map((part, i) => {
              if (part.startsWith("```")) {
                const lines = part.slice(3, -3).split("\n");
                const lang = lines[0]?.trim() || "code";
                const code = lines.slice(1).join("\n");
                return <CodeBlock key={i} lang={lang} code={code} />;
              }

              return (
                <ReactMarkdown
                  key={i}
                  remarkPlugins={[remarkGfm]}
                  components={{
                    p({ children }) {
                      return (
                        <span className="block mb-2 last:mb-0">{children}</span>
                      );
                    },
                    a({ href, children }) {
                      return (
                        <a
                          href={href}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-indigo-smoke-400 hover:text-indigo-smoke-300 underline transition-colors"
                        >
                          {children}
                        </a>
                      );
                    },
                    h1({ children }) {
                      return (
                        <h1 className="text-2xl font-bold mt-4 mb-2 text-parchment-100">
                          {children}
                        </h1>
                      );
                    },
                    h2({ children }) {
                      return (
                        <h2 className="text-xl font-bold mt-3 mb-2 text-parchment-100">
                          {children}
                        </h2>
                      );
                    },
                    h3({ children }) {
                      return (
                        <h3 className="text-lg font-bold mt-3 mb-2 text-parchment-100">
                          {children}
                        </h3>
                      );
                    },
                    h4({ children }) {
                      return (
                        <h4 className="text-base font-bold mt-2 mb-1 text-parchment-100">
                          {children}
                        </h4>
                      );
                    },
                    ul({ children }) {
                      return (
                        <ul className="list-disc ml-6 my-2 space-y-1">
                          {children}
                        </ul>
                      );
                    },
                    ol({ children }) {
                      return (
                        <ol className="list-decimal ml-6 my-2 space-y-1">
                          {children}
                        </ol>
                      );
                    },
                    li({ children }) {
                      return <li className="mb-0.5">{children}</li>;
                    },
                    blockquote({ children }) {
                      return (
                        <blockquote className="border-l-4 border-indigo-smoke-700 pl-4 my-2 italic text-warm-grey-400">
                          {children}
                        </blockquote>
                      );
                    },
                    code({ className, children, ...props }) {
                      // Inline code (not a full code block)
                      return (
                        <code
                          className="bg-slate-grey-800 rounded px-1.5 py-0.5 text-parchment-200 font-mono text-sm"
                          {...props}
                        >
                          {children}
                        </code>
                      );
                    },
                    strong({ children }) {
                      return (
                        <strong className="font-bold text-parchment-100">
                          {children}
                        </strong>
                      );
                    },
                    em({ children }) {
                      return (
                        <em className="italic text-parchment-200">
                          {children}
                        </em>
                      );
                    },
                    hr() {
                      return <hr className="my-4 border-slate-grey-700" />;
                    },
                    table({ children }) {
                      return (
                        <div className="overflow-x-auto my-3">
                          <table className="min-w-full border-collapse border border-slate-grey-700">
                            {children}
                          </table>
                        </div>
                      );
                    },
                    th({ children }) {
                      return (
                        <th className="border border-slate-grey-700 px-3 py-2 bg-slate-grey-800 font-semibold">
                          {children}
                        </th>
                      );
                    },
                    td({ children }) {
                      return (
                        <td className="border border-slate-grey-700 px-3 py-2">
                          {children}
                        </td>
                      );
                    },
                  }}
                >
                  {part}
                </ReactMarkdown>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
