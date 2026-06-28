<script lang="ts">
  import type { Snippet } from 'svelte';
  import '$lib/theme.css';

  let { children }: { children: Snippet } = $props();
</script>

{@render children()}
