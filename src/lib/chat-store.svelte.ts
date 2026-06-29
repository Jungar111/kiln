import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toMessage } from './errors';

export type ChatRole = 'user' | 'assistant';
export type ChatMessage = { readonly role: ChatRole; readonly content: string };

export type Chat = {
  readonly messages: readonly ChatMessage[];
  send(content: string): Promise<void>;
  /** Append an assistant-side note (e.g. "Run started: …") without a Claude call. */
  note(content: string): void;
};

function createChat(): Chat {
  const messages = $state<ChatMessage[]>([]);

  async function send(content: string): Promise<void> {
    messages.push({ role: 'user', content });
    // A live placeholder the `chat:delta` stream fills token by token. The
    // backend resolves with the final cleaned prose (proposal block stripped),
    // which replaces whatever streamed in.
    const i = messages.length;
    messages.push({ role: 'assistant', content: '' });
    let streamed = '';
    const unlisten = await listen<string>('chat:delta', (event) => {
      streamed += event.payload;
      messages[i] = { role: 'assistant', content: streamed };
    });
    try {
      const reply = await invoke<string>('chat', { message: content });
      messages[i] = { role: 'assistant', content: reply };
    } catch (err) {
      // The command can fail (claude not on PATH, CLI error). Surface it in the
      // transcript rather than dropping it on the floor.
      messages[i] = { role: 'assistant', content: `⚠️ ${toMessage(err)}` };
    } finally {
      unlisten();
    }
  }

  function note(content: string): void {
    messages.push({ role: 'assistant', content });
  }

  return {
    get messages() {
      return messages;
    },
    send,
    note,
  };
}

/**
 * One shared transcript for the whole app, so the premise gate can post run
 * notes into the same conversation the Chat pane renders.
 */
export const chat: Chat = createChat();
