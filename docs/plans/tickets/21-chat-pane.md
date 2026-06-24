# Ticket 21 — Chat pane connected to a `chat` Tauri command

**Phase:** 3
**Depends on:** [20](./20-app-shell.md)
**Blocks:** [22](./22-claude-client-stub.md), [30](./30-propose-experiment-schema.md)

## Goal

The Chat pane displays a transcript of `{role: 'user' | 'assistant', content: string}` messages and posts to a `chat(message)` Tauri command. The command currently echoes the message verbatim (`"you said: ..."`). Replace with a real Claude call in Ticket 22.

## Files

- Create: `src/lib/chat-store.svelte.ts`.
- Modify: `src/lib/components/ChatPane.svelte`.
- Modify: `src-tauri/src/commands.rs` — add the `chat` command stub.
- Modify: `src-tauri/src/lib.rs` — register it.

## Steps

- [ ] **1. Failing UI check.** Type in chat → nothing happens.

- [ ] **2. Rust stub.**

  ```rust
  // src-tauri/src/commands.rs
  #[tauri::command]
  pub fn chat(message: String) -> String {
      format!("you said: {message}")
  }
  ```

  Register in `invoke_handler![..., commands::chat]`.

- [ ] **3. TS chat store + component.**

  ```ts
  // src/lib/chat-store.svelte.ts
  import { invoke } from '@tauri-apps/api/core';

  export type ChatRole = 'user' | 'assistant';
  export type ChatMessage = { readonly role: ChatRole; readonly content: string };

  export function createChat(): {
    readonly messages: readonly ChatMessage[];
    send(content: string): Promise<void>;
  } {
    const messages = $state<ChatMessage[]>([]);
    async function send(content: string): Promise<void> {
      messages.push({ role: 'user', content });
      const reply = await invoke<string>('chat', { message: content });
      messages.push({ role: 'assistant', content: reply });
    }
    return {
      get messages() { return messages; },
      send,
    };
  }
  ```

  ```svelte
  <!-- src/lib/components/ChatPane.svelte -->
  <script lang="ts">
    import { createChat } from '$lib/chat-store.svelte';
    const chat = createChat();
    let draft = $state('');
    async function submit(event: SubmitEvent): Promise<void> {
      event.preventDefault();
      const content = draft.trim();
      if (!content) return;
      draft = '';
      await chat.send(content);
    }
  </script>

  <div class="chat">
    <ol class="transcript">
      {#each chat.messages as msg}
        <li class="msg msg-{msg.role}">{msg.content}</li>
      {/each}
    </ol>
    <form onsubmit={submit}>
      <textarea bind:value={draft} placeholder="Talk to Claude…"></textarea>
      <button type="submit">send</button>
    </form>
  </div>
  ```

- [ ] **4. Smoke test.** `just dev`. Type "hi". Get back "you said: hi".

- [ ] **5. Lint + commit.**

  ```sh
  just lint
  git commit -m "feat(chat): chat pane wired to a stub `chat` Tauri command"
  ```

## Acceptance

- Messages persist across sends (state held in the store, not the component).
- No `any`. Strict ESLint passes.
- No floating promises (note `void` on listeners, `await` on `chat.send`).

## Out of scope

- Markdown rendering — fast-follow.
- Streaming partial responses — Ticket 22.
- Persisting transcripts across restarts — out of MVP.
