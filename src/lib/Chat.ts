import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Message {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatHandlers {
  onToken: (chunk: string) => void;
  onDone: () => void;
  onError: (message: string) => void;
}

/**
 * Start a streaming chat completion via the Rust backend.
 * Returns an unlisten function — call it to detach the event listeners
 * (the stream itself is stopped via stopChat()).
 */
export async function startChat(
  model: string,
  messages: Message[],
  handlers: ChatHandlers,
): Promise<UnlistenFn> {
  // Attach listeners before invoking so no events are missed
  const unlistenToken = await listen<string>("chat-token", (e) =>
    handlers.onToken(e.payload),
  );
  const unlistenDone = await listen<void>("chat-done", () => {
    handlers.onDone();
    cleanup();
  });
  const unlistenError = await listen<string>("chat-error", (e) => {
    handlers.onError(e.payload);
    cleanup();
  });

  function cleanup() {
    unlistenToken();
    unlistenDone();
    unlistenError();
  }

  // Fire and forget — streaming results come back as events
  invoke("start_chat", { model, messages }).catch((err: unknown) => {
    handlers.onError(String(err));
    cleanup();
  });

  return cleanup;
}

/** Tell the Rust backend to stop after the current chunk. */
export async function stopChat(): Promise<void> {
  await invoke("stop_chat");
}

// ── Token management ──────────────────────────────────────────────────────────

/** Persist the token in the OS-backed store. Never stored in JS. */
export async function saveToken(token: string): Promise<void> {
  await invoke("save_token", { token });
}

/** Returns true if a token is already saved — does NOT return the token. */
export async function hasToken(): Promise<boolean> {
  return invoke<boolean>("has_token");
}

/** Remove the stored token. */
export async function deleteToken(): Promise<void> {
  await invoke("delete_token");
}
