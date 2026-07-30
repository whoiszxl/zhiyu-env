import { invoke } from "@tauri-apps/api/core";
import type {
  AiChatMessage,
  AiChatSession,
  AiToolCapability,
} from "../types";

export function listAiChatSessions(): Promise<AiChatSession[]> {
  return invoke<AiChatSession[]>("ai_chat_sessions_list");
}

export function createAiChatSession(): Promise<AiChatSession> {
  return invoke<AiChatSession>("ai_chat_session_create");
}

export function deleteAiChatSession(sessionId: string): Promise<void> {
  return invoke<void>("ai_chat_session_delete", { sessionId });
}

export function listAiChatMessages(
  sessionId: string,
): Promise<AiChatMessage[]> {
  return invoke<AiChatMessage[]>("ai_chat_messages_list", { sessionId });
}

export function sendAiChatMessage(
  sessionId: string,
  requestId: string,
  content: string,
): Promise<void> {
  return invoke<void>("ai_chat_send", {
    input: { sessionId, requestId, content },
  });
}

export function cancelAiChatMessage(requestId: string): Promise<void> {
  return invoke<void>("ai_chat_cancel", { requestId });
}

export function generateAiToolResult(input: {
  requestId: string;
  capability: AiToolCapability;
  instruction: string;
  context: string;
  outputLanguage: string;
}): Promise<void> {
  return invoke<void>("ai_tool_generate", { input });
}

export function cancelAiToolResult(requestId: string): Promise<void> {
  return invoke<void>("ai_tool_cancel", { requestId });
}
