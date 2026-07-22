<script lang="ts">
  import {
    Archive,
    ArchiveRestore,
    Lock,
    LockOpen,
    Sparkles,
    FolderOpen,
  } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import Panel from './lib/components/Panel.svelte';
  import Button from './lib/components/Button.svelte';
  import Input from './lib/components/Input.svelte';
  import ProgressBar from './lib/components/ProgressBar.svelte';

  type Mode = 'compress' | 'decompress' | 'encrypt' | 'decrypt';
  type LogLevel = 'info' | 'warn' | 'error';

  interface LogEntry {
    time: string;
    level: LogLevel;
    msg: string;
  }

  let mode = $state<Mode>('compress');
  let theme = $state<'dark' | 'light'>('dark');
  let inputPath = $state('');
  let outputPath = $state('');
  let format = $state('zip');
  let level = $state(6);
  let password = $state('');
  let progress = $state(0);
  let isRunning = $state(false);
  let logs = $state<LogEntry[]>([]);

  const modes: { id: Mode; label: string; icon: ComponentType }[] = [
    { id: 'compress', label: '压缩', icon: Archive },
    { id: 'decompress', label: '解压', icon: ArchiveRestore },
    { id: 'encrypt', label: '加密', icon: Lock },
    { id: 'decrypt', label: '解密', icon: LockOpen },
  ];

  let modeLabel = $derived(modes.find((m) => m.id === mode)?.label ?? '');
  let statusText = $derived(isRunning ? '处理中...' : '就绪');
  let showPassword = $derived(mode === 'encrypt' || mode === 'decrypt' || password.length > 0);
  let showFormatOptions = $derived(mode === 'compress' || mode === 'encrypt');

  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.classList.toggle('light', theme === 'light');
  }

  function selectMode(m: Mode) {
    mode = m;
  }

  function startTask() {
    isRunning = true;
    progress = 0;
    const now = new Date().toLocaleTimeString('zh-CN', { hour12: false });
    logs = [...logs, { time: now, level: 'info', msg: `开始${modeLabel}任务: ${inputPath}` }];
    // 静态布局占位: 不调用 Tauri IPC
  }

  function cancelTask() {
    isRunning = false;
    const now = new Date().toLocaleTimeString('zh-CN', { hour12: false });
    logs = [...logs, { time: now, level: 'warn', msg: '任务已取消' }];
  }

  function clearLogs() {
    logs = [];
  }
</script>

<div class="flex flex-col h-screen bg-bg text-text">
  <!-- Title Bar -->
  <TitleBar
    theme={theme}
    onToggleTheme={toggleTheme}
    onOpenSettings={() => {
      const now = new Date().toLocaleTimeString('zh-CN', { hour12: false });
      logs = [...logs, { time: now, level: 'info', msg: '设置面板 (待实现)' }];
    }}
  />

  <!-- Brand + Mode Tabs -->
  <div class="flex items-center gap-3 px-4 py-2.5 border-b border-border/50 glass flex-shrink-0">
    <div class="flex items-center gap-2 mr-4">
      <div class="w-7 h-7 rounded-mac-sm bg-accent/15 flex items-center justify-center">
        <Sparkles class="w-4 h-4 text-accent" />
      </div>
      <span class="font-bold text-base tracking-tight">SmartEx</span>
    </div>
    <div class="flex items-center gap-0.5 bg-bg-hover/60 rounded-mac p-0.5">
      {#each modes as m (m.id)}
        {@const Icon = m.icon}
        <button
          onclick={() => selectMode(m.id)}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-mac-sm text-sm font-medium transition-all duration-150 {mode === m.id
            ? 'bg-accent text-white shadow-sm'
            : 'text-text-dim hover:text-text'}"
        >
          <Icon size={15} />
          {m.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Main Content -->
  <div class="flex flex-1 gap-4 p-4 overflow-hidden min-h-0">
    <!-- Left: Parameters -->
    <div class="w-[380px] flex flex-col gap-4 overflow-y-auto flex-shrink-0">
      <Panel title="输入 / 输出">
        <div class="flex flex-col gap-3">
          <div>
            <label for="input-path" class="text-sm font-medium text-text-dim mb-1.5 block">输入路径</label>
            <div class="flex gap-2">
              <input
                id="input-path"
                bind:value={inputPath}
                placeholder="选择文件或文件夹..."
                class="flex-1 min-w-0 px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm"
              />
              <Button variant="secondary" onclick={() => {}}>
                <FolderOpen size={16} />
              </Button>
            </div>
          </div>
          <div>
            <label for="output-path" class="text-sm font-medium text-text-dim mb-1.5 block">输出路径</label>
            <div class="flex gap-2">
              <input
                id="output-path"
                bind:value={outputPath}
                placeholder="选择输出位置..."
                class="flex-1 min-w-0 px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm"
              />
              <Button variant="secondary" onclick={() => {}}>
                <FolderOpen size={16} />
              </Button>
            </div>
          </div>
        </div>
      </Panel>

      <Panel title="选项">
        <div class="flex flex-col gap-4">
          {#if showFormatOptions}
            <div>
              <label for="format-select" class="text-sm font-medium text-text-dim mb-1.5 block">格式</label>
              <select
                id="format-select"
                bind:value={format}
                class="w-full px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text focus:outline-none focus:border-accent transition-all text-sm appearance-none cursor-pointer"
              >
                <option value="zip">ZIP</option>
                <option value="7z">7Z</option>
                <option value="tar">TAR</option>
                <option value="tar.gz">TAR.GZ</option>
                <option value="tar.zst">TAR.ZST</option>
              </select>
            </div>
            <div>
              <label for="level-range" class="text-sm font-medium text-text-dim mb-1.5 flex justify-between">
                <span>压缩级别</span>
                <span class="font-mono text-accent">{level}</span>
              </label>
              <input
                id="level-range"
                type="range"
                min="0"
                max="9"
                bind:value={level}
                class="w-full accent-accent cursor-pointer"
              />
              <div class="flex justify-between text-xs text-text-dim mt-1">
                <span>存储</span>
                <span>最高</span>
              </div>
            </div>
          {/if}

          {#if showPassword}
            <Input
              label="密码"
              type="password"
              bind:value={password}
              placeholder="输入密码..."
              hint={mode === 'encrypt' ? '加密后将使用此密码保护' : '输入正确密码以解密'}
            />
          {/if}

          {#if mode === 'compress'}
            <div class="text-xs text-text-dim bg-bg-hover/50 rounded-mac-sm px-3 py-2">
              提示: SmartEx 会根据使用场景智能推荐最佳格式与级别。
            </div>
          {/if}
        </div>
      </Panel>

      <Button
        variant="primary"
        onclick={startTask}
        disabled={!inputPath || isRunning}
        loading={isRunning}
        class="w-full"
      >
        {modeLabel}
      </Button>
    </div>

    <!-- Right: Progress + Log -->
    <div class="flex-1 flex flex-col gap-4 overflow-hidden min-w-0">
      <Panel title="进度">
        <ProgressBar
          progress={progress}
          indeterminate={isRunning && progress === 0}
          message={isRunning ? '正在处理...' : '空闲'}
        />
        {#if isRunning}
          <div class="mt-3 flex justify-end">
            <Button variant="danger" onclick={cancelTask}>取消</Button>
          </div>
        {/if}
      </Panel>

      <Panel title="日志" class="flex-1 min-h-0">
        <div class="flex justify-end mb-2">
          <button
            onclick={clearLogs}
            class="text-xs text-text-dim hover:text-text transition-colors"
          >
            清空
          </button>
        </div>
        <div class="h-[calc(100%-2rem)] overflow-y-auto font-mono text-xs space-y-1 pr-1">
          {#if logs.length === 0}
            <p class="text-text-dim italic">暂无日志</p>
          {:else}
            {#each logs as log (log.time + log.msg)}
              <div class="flex gap-2 leading-relaxed">
                <span class="text-text-dim flex-shrink-0">{log.time}</span>
                <span
                  class="flex-shrink-0 font-semibold {log.level === 'error'
                    ? 'text-error'
                    : log.level === 'warn'
                      ? 'text-warn'
                      : 'text-success'}"
                >
                  [{log.level.toUpperCase()}]
                </span>
                <span class="text-text">{log.msg}</span>
              </div>
            {/each}
          {/if}
        </div>
      </Panel>
    </div>
  </div>

  <!-- Status Bar -->
  <div
    class="flex items-center justify-between px-4 py-1.5 border-t border-border/50 glass text-xs text-text-dim flex-shrink-0"
  >
    <div class="flex items-center gap-4">
      <span class="flex items-center gap-1.5">
        <span
          class="w-2 h-2 rounded-full {isRunning ? 'bg-warn' : 'bg-success'}"
        ></span>
        {statusText}
      </span>
      <span>模式: {modeLabel}</span>
    </div>
    <div class="flex items-center gap-4">
      <span>SmartEx v0.6.0</span>
    </div>
  </div>
</div>
