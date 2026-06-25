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
    {#each chat.messages as msg (msg)}
      <li class="msg msg-{msg.role}">{msg.content}</li>
    {/each}
  </ol>
  <form onsubmit={(event) => void submit(event)}>
    <textarea bind:value={draft} placeholder="Talk to Claude…"></textarea>
    <button type="submit">send</button>
  </form>
</div>

<style>
  .chat {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 8px;
  }
  .transcript {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .msg {
    padding: 6px 10px;
    border-radius: 8px;
    white-space: pre-wrap;
    max-width: 90%;
  }
  .msg-user {
    align-self: flex-end;
    background: #2d4a63;
  }
  .msg-assistant {
    align-self: flex-start;
    background: #2a2a2a;
  }
  form {
    display: flex;
    gap: 6px;
  }
  textarea {
    flex: 1;
    resize: vertical;
    min-height: 44px;
    background: #111;
    color: #e6e6e6;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 6px 8px;
    font: inherit;
  }
  button {
    background: #2d4a63;
    color: #e6e6e6;
    border: none;
    border-radius: 6px;
    padding: 0 14px;
    cursor: pointer;
  }
</style>
