<script lang="ts">
  import { LoaderCircle } from 'lucide-svelte';
  import type { Snippet } from 'svelte';

  type Variant = 'primary' | 'secondary' | 'danger';

  interface Props {
    variant?: Variant;
    loading?: boolean;
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
    class?: string;
  }

  let {
    variant = 'primary',
    loading = false,
    disabled = false,
    onclick,
    children,
    class: className = '',
  }: Props = $props();

  const variantClass: Record<Variant, string> = {
    primary: 'btn-primary',
    secondary: 'btn-secondary',
    danger: 'btn-danger',
  };

  let isDisabled = $derived(disabled || loading);
</script>

<button
  class="{variantClass[variant]} btn-ripple {className} {isDisabled
    ? 'opacity-50 cursor-not-allowed pointer-events-none'
    : 'cursor-pointer'}"
  disabled={isDisabled}
  {onclick}
>
  {#if loading}
    <LoaderCircle class="w-4 h-4 animate-spin inline-block mr-2 -mt-0.5" />
  {/if}
  {@render children?.()}
</button>
