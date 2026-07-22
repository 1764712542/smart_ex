<script lang="ts">
  import { CircleCheck, CircleX, TriangleAlert, Info, X } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';

  type ToastType = 'success' | 'error' | 'warn' | 'info';

  interface Props {
    type?: ToastType;
    message: string;
    description?: string;
    duration?: number;
    ondismiss?: () => void;
  }

  let {
    type = 'info',
    message,
    description,
    duration = 3000,
    ondismiss,
  }: Props = $props();

  let visible = $state(true);

  const icons: Record<ToastType, ComponentType> = {
    success: CircleCheck,
    error: CircleX,
    warn: TriangleAlert,
    info: Info,
  };

  const colors: Record<ToastType, string> = {
    success: 'text-success',
    error: 'text-error',
    warn: 'text-warn',
    info: 'text-accent',
  };

  let Icon = $derived(icons[type]);
  let opacityClass = $derived(visible ? 'opacity-100' : 'opacity-0');

  $effect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        visible = false;
        setTimeout(() => ondismiss?.(), 300);
      }, duration);
      return () => clearTimeout(timer);
    }
  });

  function dismiss() {
    visible = false;
    setTimeout(() => ondismiss?.(), 300);
  }
</script>

<div
  class="glass rounded-mac-lg shadow-xl border border-border/50 px-4 py-3 flex items-start gap-3 min-w-[280px] max-w-[400px] animate-scale-in transition-opacity duration-300 {opacityClass}"
>
  <Icon class="w-5 h-5 flex-shrink-0 mt-0.5 {colors[type]}" />
  <div class="flex-1 min-w-0">
    <p class="text-sm font-medium text-text">{message}</p>
    {#if description}
      <p class="text-xs text-text-dim mt-0.5">{description}</p>
    {/if}
  </div>
  <button
    onclick={dismiss}
    class="text-text-dim hover:text-text transition-colors flex-shrink-0"
    aria-label="Dismiss"
  >
    <X class="w-4 h-4" />
  </button>
</div>
