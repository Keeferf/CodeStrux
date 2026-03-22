import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { ChatMessage, CreativityKey, Session } from "./types";
import { Header, Sidebar } from "./components/layout";
import { ChatArea } from "./components/chat";
import { SettingsPanel } from "./components/settings";
import {
  getDownloadedModels,
  cancelDownload,
  onDownloadProgress,
  onDownloadDone,
  onDownloadCancelled,
  onDownloadError,
  type DownloadedModel,
  type DownloadProgress,
} from "./lib/Download";
import "./index.css";

interface LoadedModelInfo {
  model_id: string;
  filename: string;
  backend: string;
}

function createSession(title: string = "New session"): Session {
  return { id: Date.now(), title, model: "local", time: "now" };
}

export default function App() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [creativity, setCreativity] = useState<CreativityKey>("balanced");
  const [showSettings, setShowSettings] = useState(false);
  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── Loaded local model ────────────────────────────────────────────────────
  const [loadedModel, setLoadedModel] = useState<LoadedModelInfo | null>(null);

  // ── Download state ────────────────────────────────────────────────────────
  const [downloadedModels, setDownloadedModels] = useState<DownloadedModel[]>(
    [],
  );
  const [activeDownload, setActiveDownload] = useState<DownloadProgress | null>(
    null,
  );

  const chatUnlistensRef = useRef<UnlistenFn[]>([]);
  const dlUnlistenRef = useRef<UnlistenFn[]>([]);

  // ── Init ──────────────────────────────────────────────────────────────────

  useEffect(() => {
    if (!isReady && sessions.length === 0) {
      const defaultSession = createSession();
      setSessions([defaultSession]);
      setActiveSessionId(defaultSession.id);
      setIsReady(true);
    }
    refreshDownloadedModels();

    // Sync whatever model is already loaded (e.g. after a hot reload).
    invoke<LoadedModelInfo | null>("get_loaded_model")
      .then((info) => setLoadedModel(info ?? null))
      .catch(() => {});
  }, [isReady, sessions.length]);

  // ── Track loaded model via events ─────────────────────────────────────────

  useEffect(() => {
    const unlistens: Promise<UnlistenFn>[] = [
      listen<LoadedModelInfo>("model-loaded", (e) => setLoadedModel(e.payload)),
      listen("model-error", () => setLoadedModel(null)),
      // Cleared when the user explicitly unloads via SettingsPanel.
      listen("unload-model", () => setLoadedModel(null)),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  // ── Download event listeners ──────────────────────────────────────────────

  useEffect(() => {
    const setup = async () => {
      const unlistens = await Promise.all([
        onDownloadProgress((p) => setActiveDownload(p)),
        onDownloadDone(() => {
          setActiveDownload(null);
          refreshDownloadedModels();
        }),
        onDownloadCancelled(() => setActiveDownload(null)),
        onDownloadError((msg) => {
          setActiveDownload(null);
          setError(`Download failed: ${msg}`);
        }),
      ]);
      dlUnlistenRef.current = unlistens;
    };
    setup();
    return () => {
      dlUnlistenRef.current.forEach((fn) => fn());
    };
  }, []);

  const refreshDownloadedModels = async () => {
    try {
      setDownloadedModels(await getDownloadedModels());
    } catch {
      /* ignore */
    }
  };

  const handleCancelDownload = async () => {
    await cancelDownload();
    setActiveDownload(null);
  };

  // ── Session helpers ───────────────────────────────────────────────────────

  const activeSession = sessions.find((s) => s.id === activeSessionId);
  if (!activeSession)
    return <div className="bg-slate-grey-950 h-screen w-screen" />;

  const resetSession = () => {
    handleStop();
    setMessages([]);
    setError(null);
  };

  const handleNewSession = () => {
    resetSession();
    const s = createSession();
    setSessions((prev) => [s, ...prev]);
    setActiveSessionId(s.id);
  };

  const handleSelectSession = (id: number) => {
    resetSession();
    setActiveSessionId(id);
  };

  const handleDeleteSession = (id: number) => {
    setSessions((prev) => {
      const remaining = prev.filter((s) => s.id !== id);
      if (remaining.length === 0) {
        const replacement = createSession();
        setActiveSessionId(replacement.id);
        setMessages([]);
        return [replacement];
      }
      if (activeSessionId === id) {
        const idx = prev.findIndex((s) => s.id === id);
        setActiveSessionId(remaining[Math.min(idx, remaining.length - 1)].id);
        setMessages([]);
      }
      return remaining;
    });
  };

  // ── Chat handlers ─────────────────────────────────────────────────────────

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;

    // Guard: a model must be loaded in llama-server before chatting.
    if (!loadedModel) {
      setError(
        "Please load a model first — open Settings and click ▶ on a downloaded model.",
      );
      return;
    }

    setError(null);

    const userMsg: ChatMessage = {
      id: Date.now(),
      role: "user",
      content: input.trim(),
    };
    const assistantId = Date.now() + 1;
    const assistantMsg: ChatMessage = {
      id: assistantId,
      role: "assistant",
      content: "",
    };

    setMessages((prev) => [...prev, userMsg, assistantMsg]);
    setInput("");
    setIsLoading(true);

    const history = [...messages, userMsg]
      .filter((m) => m.role !== "system")
      .map((m) => ({
        role: m.role as "user" | "assistant",
        content: m.content,
      }));

    // Clean up any previous listeners before registering new ones.
    chatUnlistensRef.current.forEach((fn) => fn());
    chatUnlistensRef.current = [];

    const unlistens = await Promise.all([
      listen<string>("local-chat-token", (e) => {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === assistantId ? { ...m, content: m.content + e.payload } : m,
          ),
        );
      }),
      listen("local-chat-done", () => {
        setIsLoading(false);
        chatUnlistensRef.current.forEach((fn) => fn());
        chatUnlistensRef.current = [];
      }),
      listen<string>("local-chat-error", (e) => {
        setError(e.payload);
        setIsLoading(false);
        setMessages((prev) => prev.filter((m) => m.id !== assistantId));
        chatUnlistensRef.current.forEach((fn) => fn());
        chatUnlistensRef.current = [];
      }),
    ]);

    chatUnlistensRef.current = unlistens;

    try {
      await invoke("start_local_chat", { messages: history });
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start chat.");
      setIsLoading(false);
      setMessages((prev) => prev.filter((m) => m.id !== assistantId));
      chatUnlistensRef.current.forEach((fn) => fn());
      chatUnlistensRef.current = [];
    }
  };

  const handleStop = async () => {
    await invoke("stop_local_chat").catch(() => {});
    chatUnlistensRef.current.forEach((fn) => fn());
    chatUnlistensRef.current = [];
    setIsLoading(false);
  };

  const downloadedModelIds = [
    ...new Set(downloadedModels.map((m) => m.model_id)),
  ];

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden overscroll-none bg-slate-grey-950 text-parchment-300">
      <Header
        model={loadedModel?.model_id ?? ""}
        onModelChange={() => {}}
        downloadedModelIds={downloadedModelIds}
        onDownloadStart={() => {}}
      />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          sessions={sessions}
          activeSessionId={activeSessionId}
          onSelectSession={handleSelectSession}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
          showSettings={showSettings}
          onToggleSettings={() => setShowSettings((p) => !p)}
        />
        <ChatArea
          activeSession={activeSession}
          messages={messages}
          input={input}
          creativity={creativity}
          onInputChange={setInput}
          onSend={handleSend}
          onStop={handleStop}
          isLoading={isLoading}
          error={error}
        />
        {showSettings && (
          <SettingsPanel
            creativity={creativity}
            downloadedModels={downloadedModels}
            activeDownload={activeDownload}
            onCreativityChange={setCreativity}
            onCancelDownload={handleCancelDownload}
            onModelsChanged={refreshDownloadedModels}
          />
        )}
      </div>
    </div>
  );
}
