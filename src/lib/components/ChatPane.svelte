<script lang="ts">
  import { chat } from '$lib/chat-store.svelte';
  import type { SidecarStatus } from '$lib/sidecar-status.svelte';

  let { status = 'starting' }: { status?: SidecarStatus } = $props();

  let draft = $state('');

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const content = draft.trim();
    if (!content) return;
    draft = '';
    await chat.send(content);
  }

  function onkeydown(event: KeyboardEvent): void {
    // ⌘↵ / Ctrl↵ sends; plain Enter inserts a newline.
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      void submit(new SubmitEvent('submit'));
    }
  }
</script>

<div class="chat">
  <header>
    <span class="avatar">▲</span>
    <span class="who">Claude</span>
    {#if status === 'ready'}
      <span class="tag good">● shared kernel</span>
    {:else}
      <span class="tag">flight director: you</span>
    {/if}
  </header>

  <ol class="transcript">
    {#each chat.messages as msg (msg)}
      {#if msg.role === 'assistant'}
        <li class="turn">
          <div class="byline"><span class="avatar sm">▲</span><span class="name">Claude</span></div>
          <div class="said">{msg.content}</div>
        </li>
      {:else}
        <li class="user">{msg.content}</li>
      {/if}
    {/each}
  </ol>

  <form onsubmit={(event) => void submit(event)}>
    <div class="box">
      <textarea bind:value={draft} {onkeydown} placeholder="Direct Claude…"></textarea>
      <button type="submit" class:active={draft.trim() !== ''} aria-label="Send">↑</button>
    </div>
  </form>
</div>

<style>
  .chat {
    width: 344px;
    flex-shrink: 0;
    background: var(--bg-chat);
    border-left: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    height: 40px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    padding: 0 14px;
    gap: 8px;
  }
  .avatar {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background: var(--logo-grad);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 8px;
    color: #1a1208;
    flex-shrink: 0;
  }
  .who {
    color: var(--tx-bright);
    font-weight: 600;
    font-size: 12px;
  }
  .tag {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--tx-mut2);
  }
  .tag.good {
    color: var(--good);
  }
  .transcript {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 16px 14px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .byline {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 6px;
  }
  .avatar.sm {
    width: 16px;
    height: 16px;
  }
  .name {
    font-size: 11px;
    color: var(--tx-dim2);
    font-family: var(--font-mono);
  }
  .said {
    color: var(--tx-2);
    font-size: 12.5px;
    white-space: pre-wrap;
  }
  .user {
    align-self: flex-end;
    max-width: 84%;
    background: #262320;
    border: 1px solid var(--bd-2);
    border-radius: 10px 10px 2px 10px;
    padding: 9px 11px;
    color: var(--tx-3);
    font-size: 12.5px;
    white-space: pre-wrap;
  }
  form {
    padding: 12px 14px;
    border-top: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .box {
    background: var(--bg-input);
    border: 1px solid var(--bd-2);
    border-radius: 8px;
    padding: 8px 10px;
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }
  .box:focus-within {
    border-color: var(--bd-ember);
  }
  textarea {
    flex: 1;
    resize: none;
    min-height: 22px;
    max-height: 160px;
    background: transparent;
    color: var(--tx-3);
    border: none;
    outline: none;
    padding: 0;
    font: inherit;
    font-size: 12px;
  }
  textarea::placeholder {
    color: var(--tx-mut2);
  }
  button {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    border-radius: 5px;
    background: #262320;
    color: var(--tx-dim2);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  button:hover {
    color: var(--tx-bright);
  }
  button.active {
    background: var(--ember);
    color: #1a1208;
  }
  button.active:hover {
    background: var(--ember-soft);
    color: #1a1208;
  }
</style>
