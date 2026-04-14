export type MessageRole = "user" | "assistant" | "system";

export interface ChatMessage {
  id: number;
  role: MessageRole;
  content: string;
}

export interface Session {
  id: number;
  title: string;
  model: string;
  time: string;
}
