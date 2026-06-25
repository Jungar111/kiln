import { invoke } from '@tauri-apps/api/core';

export type ChatRole = 'user' | 'assistant';
export type ChatMessage = { readonly role: ChatRole; readonly content: string };

export type Chat = {
  readonly messages: readonly ChatMessage[];
  send(content: string): Promise<void>;
};

/**
 * Reactive chat transcript backed by the `chat` Tauri command. State lives here
 * (not in the component) so messages survive component re-renders.
 */
export function createChat(): Chat {
  const messages = $state<ChatMessage[]>([]);

  async function send(content: string): Promise<void> {
    messages.push({ role: 'user', content });
    try {
      const reply = await invoke<string>('chat', { message: content });
      messages.push({ role: 'assistant', content: reply });
    } catch (err) {
      // The command can fail (claude not on PATH, CLI error). Surface it in the
      // transcript rather than dropping it on the floor.
      const detail = err instanceof Error ? err.message : String(err);
      messages.push({ role: 'assistant', content: `⚠️ ${detail}` });
    }
  }

  return {
    get messages() {
      return messages;
    },
    send,
  };
}
