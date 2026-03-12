import { useState, useEffect } from "react";
import type { ChatMessage, CreativityKey, Session } from "./types";
import { Header, Sidebar } from "./components/layout";
import { ChatArea } from "./components/chat";
import { SettingsPanel } from "./components/settings";
import "./index.css";

function createSession(model: string, title: string = "New session"): Session {
  return {
    id: Date.now(),
    title,
    model: model || "none",
    time: "now",
  };
}

export default function App() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState<string>("");
  const [availableModels] = useState<string[]>([]);
  const [model, setModel] = useState<string>("");
  const [creativity, setCreativity] = useState<CreativityKey>("balanced");
  const [showSettings, setShowSettings] = useState<boolean>(true);
  const [isReady, setIsReady] = useState<boolean>(false);

  useEffect(() => {
    if (!isReady && sessions.length === 0) {
      const defaultSession = createSession("");
      setSessions([defaultSession]);
      setActiveSessionId(defaultSession.id);
      setIsReady(true);
    }
  }, [isReady, sessions.length]);

  const activeSession = sessions.find((s) => s.id === activeSessionId);
  if (!activeSession) {
    return <div className="bg-slate-grey-950 h-screen w-screen" />;
  }

  const handleSend = () => {
    if (!input.trim() || activeSessionId === null) return;
    setMessages((prev) => [
      ...prev,
      { id: Date.now(), role: "user", content: input.trim() },
    ]);
    setInput("");
  };

  const handleNewSession = () => {
    const newSession = createSession(model);
    setSessions((prev) => [newSession, ...prev]);
    setActiveSessionId(newSession.id);
    setMessages([]);
  };

  const handleSelectSession = (id: number) => {
    setActiveSessionId(id);
    setMessages([]);
  };

  const handleDeleteSession = (id: number) => {
    setSessions((prev) => {
      const remaining = prev.filter((s) => s.id !== id);

      if (remaining.length === 0) {
        const replacement = createSession(model);
        setActiveSessionId(replacement.id);
        setMessages([]);
        return [replacement];
      }

      if (activeSessionId === id) {
        const deletedIndex = prev.findIndex((s) => s.id === id);
        const nextIndex = Math.min(deletedIndex, remaining.length - 1);
        setActiveSessionId(remaining[nextIndex].id);
        setMessages([]);
      }

      return remaining;
    });
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-slate-grey-950 text-parchment-300">
      <Header model={model} onModelChange={setModel} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          sessions={sessions}
          activeSessionId={activeSessionId}
          onSelectSession={handleSelectSession}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
          showSettings={showSettings}
          onToggleSettings={() => setShowSettings((prev) => !prev)}
        />
        <ChatArea
          activeSession={activeSession}
          messages={messages}
          input={input}
          creativity={creativity}
          onInputChange={setInput}
          onSend={handleSend}
        />
        {showSettings && (
          <SettingsPanel
            model={model}
            availableModels={availableModels}
            creativity={creativity}
            onModelChange={setModel}
            onCreativityChange={setCreativity}
          />
        )}
      </div>
    </div>
  );
}
