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

  // 后端发送 0.0~1.0 的浮点数, 转换为 0~100 的百分比
  let normalizedProgress = $derived(progress > 1 ? progress : progress * 100);
  let percentage = $derived(Math.min(100, Math.max(0, Math.round(normalizedProgress))));
  let bytesText = $derived(
    bytesDone != null && bytesTotal != null && bytesTotal > 0
      ? `${formatBytes(bytesDone)} / ${formatBytes(bytesTotal)}`
      : bytesDone != null && bytesDone > 0
        ? formatBytes(bytesDone)
        : ''
  );
  let isComplete = $derived(percentage >= 100);

  // ===== 速度与剩余时间计算 =====
  let speed = $state(0); // bytes/sec
  let eta = $state(0); // seconds
  let lastUpdate = $state(0);
  let lastBytes = $state(0);

  $effect(() => {
    // 依赖 progress 和 bytesDone 触发
    const _progress = progress;
    const _bytes = bytesDone ?? 0;
    const now = Date.now();

    if (lastUpdate === 0) {
      // 首次记录
      lastUpdate = now;
      lastBytes = _bytes;
      return;
    }

    const elapsed = (now - lastUpdate) / 1000; // seconds
    if (elapsed < 0.3) return; // 至少 300ms 才更新一次, 避免抖动

    const bytesDiff = _bytes - lastBytes;
    if (bytesDiff > 0 && elapsed > 0) {
      const instantSpeed = bytesDiff / elapsed;
      // 平滑速度: 70% 旧值 + 30% 新值
      speed = speed === 0 ? instantSpeed : speed * 0.7 + instantSpeed * 0.3;
    }

    // 预估剩余时间
    if (speed > 0 && bytesTotal && bytesTotal > 0 && _bytes < bytesTotal) {
      eta = (bytesTotal - _bytes) / speed;
    }

    lastUpdate = now;
    lastBytes = _bytes;
  });

  // 重置: 当 progress 回到 0 时清空速度
  $effect(() => {
    if (progress === 0) {
      speed = 0;
      eta = 0;
      lastUpdate = 0;
      lastBytes = 0;
    }
  });

  function formatSpeed(bps: number): string {
    if (bps <= 0) return '';
    return `${formatBytes(bps)}/s`;
  }

  function formatETA(s: number): string {
    if (s <= 0) return '';
    if (s < 60) return `${Math.round(s)}s`;
    const m = Math.floor(s / 60);
    const r = Math.round(s % 60);
    return `${m}m ${r}s`;
  }

  let speedText = $derived(formatSpeed(speed));
  let etaText = $derived(formatETA(eta));
</script>

<div class="flex flex-col gap-1.5">
  {#if message || bytesText || speedText}
    <div class="flex justify-between items-center text-xs text-text-dim gap-2">
      {#if message}
        <span class="truncate">{message}</span>
      {:else}
        <span></span>
      {/if}
      <div class="flex items-center gap-3 font-mono text-[11px] flex-shrink-0">
        {#if speedText}
          <span class="text-accent-dim">{speedText}</span>
        {/if}
        {#if etaText && !isComplete}
          <span class="text-text-dim">ETA {etaText}</span>
        {/if}
        {#if bytesText}
          <span>{bytesText}</span>
        {/if}
      </div>
    </div>
  {/if}
  <div class="relative h-2.5 rounded-full bg-bg-hover overflow-hidden">
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
    <div class="flex justify-between items-center text-xs font-mono text-text-dim">
      <span>{isComplete ? '已完成' : ''}</span>
      <span>{percentage}%</span>
    </div>
  {/if}
</div>

<style>
  @keyframes indeterminate {
    0% { left: -33%; }
    100% { left: 100%; }
  }
</style>
