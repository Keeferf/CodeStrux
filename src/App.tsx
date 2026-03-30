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

interface StoredConversation {
  id: string;
  model_id: string;
  model_filename: string;
  backend: string;
  title: string;
  created_at: number;
  updated_at: number;
}

interface StoredMessage {
  id: number;
  conversation_id: string;
  role: string;
  content: string;
  created_at: number;
}

function convToSession(c: StoredConversation): Session {
  return {
    id: c.id as unknown as number,
    title: c.title,
    model: c.model_id || "local",
    time: new Date(c.created_at * 1000).toLocaleDateString(),
  };
}

export default function App() {
  const sessionDbId = useRef<Map<number, string>>(new Map());
  const tempIdCounter = useRef(0);

  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [creativity, setCreativity] = useState<CreativityKey>("balanced");
  const [showSettings, setShowSettings] = useState(false);
  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [loadedModel, setLoadedModel] = useState<LoadedModelInfo | null>(null);
  const [downloadedModels, setDownloadedModels] = useState<DownloadedModel[]>(
    [],
  );
  const [activeDownload, setActiveDownload] = useState<DownloadProgress | null>(
    null,
  );

  const activeConvId = useRef<string | null>(null);
  const streamedReply = useRef("");

  const chatUnlistensRef = useRef<UnlistenFn[]>([]);
  const dlUnlistenRef = useRef<UnlistenFn[]>([]);

  // ── Init ──────────────────────────────────────────────────────────────────

  useEffect(() => {
    if (isReady) return;
    setIsReady(true);

    refreshDownloadedModels();

    invoke<LoadedModelInfo | null>("get_loaded_model")
      .then((info) => setLoadedModel(info ?? null))
      .catch(() => {});

    invoke<StoredConversation[]>("list_conversations")
      .then((convs) => {
        if (convs.length === 0) {
          const tempId = --tempIdCounter.current;
          setSessions([
            { id: tempId, title: "New session", model: "local", time: "now" },
          ]);
          setActiveSessionId(tempId);
        } else {
          const mapped = convs.map(convToSession);
          convs.forEach((c) =>
            sessionDbId.current.set(c.id as unknown as number, c.id),
          );
          setSessions(mapped);
          setActiveSessionId(mapped[0].id);
          activeConvId.current = convs[0].id;
          loadMessagesForConv(convs[0].id);
        }
      })
      .catch(() => {
        const tempId = --tempIdCounter.current;
        setSessions([
          { id: tempId, title: "New session", model: "local", time: "now" },
        ]);
        setActiveSessionId(tempId);
      });
  }, [isReady]);

  const loadMessagesForConv = async (convId: string) => {
    try {
      const stored = await invoke<StoredMessage[]>(
        "get_conversation_messages",
        {
          conversationId: convId,
        },
      );
      setMessages(
        stored.map((m) => ({
          id: m.id,
          role: m.role as ChatMessage["role"],
          content: m.content,
        })),
      );
    } catch {
      setMessages([]);
    }
  };

  // ── Model events ──────────────────────────────────────────────────────────

  useEffect(() => {
    const unlistens: Promise<UnlistenFn>[] = [
      listen<LoadedModelInfo>("model-loaded", (e) => setLoadedModel(e.payload)),
      listen("model-error", () => setLoadedModel(null)),
      listen("unload-model", () => setLoadedModel(null)),
    ];
    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  // ── Download events ───────────────────────────────────────────────────────

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
    activeConvId.current = null;
    streamedReply.current = "";
  };

  const handleNewSession = () => {
    resetSession();
    const tempId = --tempIdCounter.current;
    const s: Session = {
      id: tempId,
      title: "New session",
      model: "local",
      time: "now",
    };
    setSessions((prev) => [s, ...prev]);
    setActiveSessionId(tempId);
  };

  const handleSelectSession = async (id: number) => {
    resetSession();
    setActiveSessionId(id);
    const dbId = sessionDbId.current.get(id);
    if (dbId) {
      activeConvId.current = dbId;
      await loadMessagesForConv(dbId);
    }
  };

  const handleDeleteSession = (id: number) => {
    const dbId = sessionDbId.current.get(id);
    if (dbId) {
      invoke("delete_conversation", { conversationId: dbId }).catch(() => {});
      sessionDbId.current.delete(id);
    }

    setSessions((prev) => {
      const remaining = prev.filter((s) => s.id !== id);
      if (remaining.length === 0) {
        const tempId = --tempIdCounter.current;
        const replacement: Session = {
          id: tempId,
          title: "New session",
          model: "local",
          time: "now",
        };
        setActiveSessionId(replacement.id);
        setMessages([]);
        activeConvId.current = null;
        return [replacement];
      }
      if (activeSessionId === id) {
        const idx = prev.findIndex((s) => s.id === id);
        const next = remaining[Math.min(idx, remaining.length - 1)];
        setActiveSessionId(next.id);
        setMessages([]);
        const nextDbId = sessionDbId.current.get(next.id) ?? null;
        activeConvId.current = nextDbId;
        if (nextDbId) loadMessagesForConv(nextDbId);
      }
      return remaining;
    });
  };

  // ── Chat handlers ─────────────────────────────────────────────────────────

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;

    if (!loadedModel) {
      setError(
        "Please load a model first — open Settings and click ▶ on a downloaded model.",
      );
      return;
    }

    setError(null);
    const userText = input.trim();
    setInput("");

    // ── 1. Ensure a DB conversation exists ────────────────────────────────
    const isFirstMessage = !activeConvId.current;
    let convId = activeConvId.current;
    if (!convId) {
      try {
        const conv = await invoke<StoredConversation>("create_conversation", {
          args: {
            model_id: loadedModel.model_id,
            model_filename: loadedModel.filename,
            backend: loadedModel.backend,
          },
        });
        convId = conv.id;
        activeConvId.current = convId;
        if (activeSessionId !== null) {
          sessionDbId.current.set(activeSessionId, convId);
        }
      } catch {
        setError("Could not create conversation record.");
        return;
      }
    }

    // ── 2. Persist user message ───────────────────────────────────────────
    try {
      const savedUser = await invoke<StoredMessage>("append_message", {
        conversationId: convId,
        role: "user",
        content: userText,
      });

      // Title: first 60 chars of the first user message, applied immediately.
      if (isFirstMessage) {
        const title = userText.slice(0, 60);
        invoke("rename_conversation", { conversationId: convId, title }).catch(
          () => {},
        );
        setSessions((prev) =>
          prev.map((s) => (s.id === activeSessionId ? { ...s, title } : s)),
        );
      }

      const userMsg: ChatMessage = {
        id: savedUser.id,
        role: "user",
        content: userText,
      };
      const assistantPlaceholderId = Date.now();
      const assistantMsg: ChatMessage = {
        id: assistantPlaceholderId,
        role: "assistant",
        content: "",
      };

      setMessages((prev) => [...prev, userMsg, assistantMsg]);
      setIsLoading(true);
      streamedReply.current = "";

      const history = [...messages, userMsg]
        .filter((m) => m.role !== "system")
        .map((m) => ({
          role: m.role as "user" | "assistant",
          content: m.content,
        }));

      // ── 3. SSE listeners ─────────────────────────────────────────────────
      chatUnlistensRef.current.forEach((fn) => fn());
      chatUnlistensRef.current = [];

      const currentConvId = convId;

      const unlistens = await Promise.all([
        listen<string>("local-chat-token", (e) => {
          streamedReply.current += e.payload;
          setMessages((prev) =>
            prev.map((m) =>
              m.id === assistantPlaceholderId
                ? { ...m, content: m.content + e.payload }
                : m,
            ),
          );
        }),

        listen("local-chat-done", async () => {
          setIsLoading(false);
          chatUnlistensRef.current.forEach((fn) => fn());
          chatUnlistensRef.current = [];

          // ── 4. Persist assistant reply ────────────────────────────────────
          if (streamedReply.current.trim()) {
            try {
              const savedAssistant = await invoke<StoredMessage>(
                "append_message",
                {
                  conversationId: currentConvId,
                  role: "assistant",
                  content: streamedReply.current,
                },
              );
              setMessages((prev) =>
                prev.map((m) =>
                  m.id === assistantPlaceholderId
                    ? { ...m, id: savedAssistant.id }
                    : m,
                ),
              );
            } catch {
              // Non-fatal
            }
          }

          streamedReply.current = "";
        }),

        listen<string>("local-chat-error", (e) => {
          setError(e.payload);
          setIsLoading(false);
          setMessages((prev) =>
            prev.filter((m) => m.id !== assistantPlaceholderId),
          );
          chatUnlistensRef.current.forEach((fn) => fn());
          chatUnlistensRef.current = [];
          streamedReply.current = "";
        }),
      ]);

      chatUnlistensRef.current = unlistens;

      await invoke("start_local_chat", { messages: history });
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start chat.");
      setIsLoading(false);
      chatUnlistensRef.current.forEach((fn) => fn());
      chatUnlistensRef.current = [];
      streamedReply.current = "";
    }
  };

  const handleStop = async () => {
    await invoke("stop_local_chat").catch(() => {});
    chatUnlistensRef.current.forEach((fn) => fn());
    chatUnlistensRef.current = [];
    setIsLoading(false);
    streamedReply.current = "";
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
