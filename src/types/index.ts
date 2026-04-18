import { AttachedFile } from "../components/chat/FileAttachment";

export type MessageRole = "user" | "assistant" | "system";

export interface ChatMessage {
  id: number;
  role: MessageRole;
  content: string;
  attachments?: AttachedFile[];
}

export interface Session {
  id: number;
  title: string;
  model: string;
  time: string;
}
