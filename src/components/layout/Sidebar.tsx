import { useState } from "react";
import {
  LuPlus,
  LuCpu,
  LuX,
  LuSettings,
  LuPanelLeftClose,
  LuPanelLeftOpen,
} from "react-icons/lu";
import type { Session } from "../../types";

interface SidebarProps {
  sessions: Session[];
  activeSessionId: number | null;
  onSelectSession: (id: number) => void;
  onNewSession: () => void;
  onDeleteSession: (id: number) => void;
  showSettings: boolean;
  onToggleSettings: () => void;
}

export function Sidebar({
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
  onDeleteSession,
  showSettings,
  onToggleSettings,
}: SidebarProps) {
  const [collapsed, setCollapsed] = useState(false);

  if (collapsed) {
    return (
      <aside className="w-12 shrink-0 flex flex-col items-center bg-slate-grey-900 border-r border-slate-grey-800 overflow-hidden py-2 gap-1">
        {/* Expand button */}
        <button
          onClick={() => setCollapsed(false)}
          className="flex items-center justify-center w-8 h-8 rounded-md text-slate-grey-500 hover:text-parchment-300 hover:bg-slate-grey-800 transition-all duration-150 cursor-pointer"
          aria-label="Expand sidebar"
          title="Expand sidebar"
        >
          <LuPanelLeftOpen size={15} />
        </button>

        {/* New session button */}
        <button
          onClick={onNewSession}
          className="flex items-center justify-center w-8 h-8 rounded-md text-slate-grey-500 hover:text-indigo-smoke-400 hover:bg-slate-grey-800 transition-all duration-150 cursor-pointer border border-slate-grey-800 hover:border-indigo-smoke-700"
          aria-label="Create new session"
          title="New session"
        >
          <LuPlus size={15} />
        </button>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Settings button */}
        <button
          onClick={onToggleSettings}
          className={`flex items-center justify-center w-8 h-8 rounded-md transition-all duration-150 cursor-pointer mb-0.5 ${
            showSettings
              ? "bg-indigo-smoke-900/50 text-indigo-smoke-400"
              : "text-slate-grey-500 hover:text-parchment-300 hover:bg-slate-grey-800"
          }`}
          aria-label="Settings"
          title="Settings"
        >
          <LuSettings size={14} />
        </button>
      </aside>
    );
  }

  return (
    <aside className="w-55 shrink-0 flex flex-col bg-slate-grey-900 border-r border-slate-grey-800 overflow-hidden">
      <div className="flex-1 overflow-y-auto p-2">
        <div className="flex justify-between items-center mb-2 px-1">
          <span className="font-display text-[10px] uppercase tracking-[1.2px] text-slate-grey-500">
            sessions
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={onNewSession}
              className="flex items-center justify-center w-7 h-7 bg-transparent border border-slate-grey-800 rounded-md cursor-pointer text-slate-grey-500 hover:text-indigo-smoke-400 hover:border-indigo-smoke-700 hover:bg-slate-grey-950 transition-all duration-150"
              aria-label="Create new session"
              title="New session"
            >
              <LuPlus size={14} />
            </button>

            {/* Collapse button */}
            <button
              onClick={() => setCollapsed(true)}
              className="flex items-center justify-center w-7 h-7 rounded-md text-slate-grey-500 hover:text-parchment-300 hover:bg-slate-grey-800 transition-all duration-150 cursor-pointer"
              aria-label="Collapse sidebar"
              title="Collapse sidebar"
            >
              <LuPanelLeftClose size={14} />
            </button>
          </div>
        </div>

        {sessions.length === 0 && (
          <div className="py-5 px-1 text-center leading-loose">
            <span className="font-body text-[13px] text-slate-grey-500">
              no sessions yet
            </span>
            <br />
            <span className="font-body text-xs text-slate-grey-600">
              press <span className="text-parchment-400 font-mono">+ new</span>{" "}
              to create one
            </span>
          </div>
        )}

        {sessions.map((session) => {
          const isActive = session.id === activeSessionId;
          return (
            <div
              key={session.id}
              className={`group relative flex items-center rounded-md border-l-2 mb-0.5 transition-all duration-150 ${
                isActive
                  ? "bg-indigo-smoke-950/60 border-l-indigo-smoke-500"
                  : "bg-transparent border-l-transparent hover:bg-indigo-smoke-950/40 hover:border-l-indigo-smoke-700"
              }`}
            >
              <button
                onClick={() => onSelectSession(session.id)}
                className="flex-1 min-w-0 text-left px-2.5 py-2 cursor-pointer"
              >
                <div
                  className={`font-body text-[13px] truncate pr-4 ${isActive ? "text-parchment-200" : "text-parchment-400"}`}
                >
                  {session.title}
                </div>
                <div className="font-mono text-[10px] mt-0.5 text-slate-grey-500">
                  {session.model} · {session.time}
                </div>
              </button>

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDeleteSession(session.id);
                }}
                className="opacity-0 group-hover:opacity-100 shrink-0 mr-1.5 p-1 rounded text-slate-grey-500 hover:text-brick-red-400 hover:bg-brick-red-950/40 transition-all duration-150 cursor-pointer"
                aria-label="Delete session"
              >
                <LuX size={11} />
              </button>
            </div>
          );
        })}
      </div>

      {/* Footer with session count and settings */}
      <div className="flex items-center justify-between px-3 py-2.5 border-t border-slate-grey-800">
        <div className="flex items-center gap-1.5 font-display text-[11px] text-slate-grey-500">
          <LuCpu size={12} />
          <span>
            {sessions.length} session{sessions.length !== 1 ? "s" : ""}
          </span>
        </div>

        <button
          onClick={onToggleSettings}
          className="flex items-center justify-center w-7 h-7 rounded-md cursor-pointer transition-all duration-150 text-slate-grey-500 hover:text-parchment-300 hover:bg-slate-grey-800"
          aria-label="Settings"
          title="Settings"
        >
          <LuSettings size={14} />
        </button>
      </div>
    </aside>
  );
}
