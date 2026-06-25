<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

{@render children()}

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    background: #1e1e1e;
    font-family:
      system-ui,
      -apple-system,
      sans-serif;
  }
  :global(*) {
    box-sizing: border-box;
  }
</style>
