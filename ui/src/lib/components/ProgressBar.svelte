<script lang="ts">
  interface Props {
    progress?: number;
    bytesDone?: number;
    bytesTotal?: number;
    message?: string;
    indeterminate?: boolean;
  }

  let {
    progress = 0,
    bytesDone,
    bytesTotal,
    message,
    indeterminate = false,
  }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  let percentage = $derived(Math.min(100, Math.max(0, Math.round(progress))));
  let bytesText = $derived(
    bytesDone != null && bytesTotal != null
      ? `${formatBytes(bytesDone)} / ${formatBytes(bytesTotal)}`
      : bytesDone != null
        ? formatBytes(bytesDone)
        : ''
  );
  let isComplete = $derived(percentage >= 100);
</script>

<div class="flex flex-col gap-1.5">
  {#if message || bytesText}
    <div class="flex justify-between items-center text-xs text-text-dim">
      {#if message}
        <span>{message}</span>
      {:else}
        <span></span>
      {/if}
      {#if bytesText}
        <span class="font-mono">{bytesText}</span>
      {/if}
    </div>
  {/if}
  <div class="relative h-2 rounded-full bg-bg-hover overflow-hidden">
    {#if indeterminate}
      <div
        class="absolute h-full w-1/3 rounded-full bg-accent"
        style="animation: indeterminate 1.5s ease-in-out infinite;"
      ></div>
    {:else}
      <div
        class="absolute h-full rounded-full transition-all duration-300 ease-out progress-shimmer {isComplete ? 'bg-success' : 'bg-accent'}"
        style="width: {percentage}%; left: 0;"
      ></div>
    {/if}
  </div>
  {#if !indeterminate}
    <div class="text-right text-xs font-mono text-text-dim">{percentage}%</div>
  {/if}
</div>

<style>
  @keyframes indeterminate {
    0% { left: -33%; }
    100% { left: 100%; }
  }
</style>
