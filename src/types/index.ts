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

export type CreativityKey = "precise" | "balanced" | "creative";

export interface CreativityMode {
  label: string;
  temp: number;
  desc: string;
  accent: string;
}

export type CreativityModes = Record<CreativityKey, CreativityMode>;
