<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    class?: string;
    children?: Snippet;
  }

  let { title, class: className = '', children }: Props = $props();
</script>

<div
  class="glass rounded-mac-lg shadow-lg border border-border/50 flex flex-col {className}"
>
  {#if title}
    <div class="px-5 py-3 border-b border-border/50 font-semibold text-sm text-text">
      {title}
    </div>
  {/if}
  <div class="p-5 flex-1">
    {@render children?.()}
  </div>
</div>
