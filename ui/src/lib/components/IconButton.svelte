<script lang="ts">
  import type { ComponentType } from 'svelte';

  interface Props {
    icon: ComponentType;
    size?: number;
    onclick?: (e: MouseEvent) => void;
    label?: string;
    active?: boolean;
    disabled?: boolean;
  }

  let {
    icon: Icon,
    size = 18,
    onclick,
    label = '',
    active = false,
    disabled = false,
  }: Props = $props();
</script>

<button
  {onclick}
  disabled={disabled}
  title={label}
  aria-label={label}
  class="p-2 rounded-mac-sm transition-all duration-150 active:scale-90 {active
    ? 'bg-bg-hover text-accent'
    : 'text-text-dim hover:text-text hover:bg-bg-hover'} {disabled
    ? 'opacity-40 cursor-not-allowed pointer-events-none'
    : 'cursor-pointer'}"
>
  <Icon {size} />
</button>
