<script lang="ts">
  // SmartEx 主界面 — Svelte 5 runes 完整版
  // 四大模式: 压缩 / 解压 / 加密 / 解密
  // 痛点①: 上下文感知压缩向导
  // 痛点②: 钥匙串集成
  import {
    Archive,
    ArchiveRestore,
    Lock,
    LockOpen,
    Sparkles,
    FolderOpen,
    FileInput,
    Eye,
    EyeOff,
    KeyRound,
    Save,
    Trash2,
    ChevronDown,
    Upload,
    Eraser,
    CircleCheck,
    CircleX,
    TriangleAlert,
    Info,
  } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import Panel from '$lib/components/Panel.svelte';
  import Button from '$lib/components/Button.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import ContextWizard from '$lib/components/ContextWizard.svelte';
  import { api } from '$lib/tauri';
  import {
    appState,
    setMode,
    toggleTheme,
    initTheme,
    pushLog,
    clearLogs,
    showToast,
    dismissToast,
    pickInputFile,
    pickOutputFile,
    pickOutputFolder,
    autoFillOutput,
    openContextWizard,
    keychainSavePassword,
    keychainLoadPassword,
    keychainDeletePassword,
    setWorking,
    resetProgress,
    canStart,
    levelRange,
    COMPRESS_FORMATS,
    type Mode,
    type LogEntry,
  } from '$lib/stores/app.svelte';
  import { startDragDrop } from '$lib/stores/dragdrop.svelte';
  import { startProgressListener } from '$lib/stores/progress.svelte';

  // ===== 模式定义 =====
  const modes: { id: Mode; label: string; icon: ComponentType; verb: string }[] = [
    { id: 'compress', label: '压缩', icon: Archive, verb: '开始压缩' },
    { id: 'decompress', label: '解压', icon: ArchiveRestore, verb: '开始解压' },
    { id: 'encrypt', label: '加密', icon: Lock, verb: '开始加密' },
    { id: 'decrypt', label: '解密', icon: LockOpen, verb: '开始解密' },
  ];

  let modeLabel = $derived(modes.find((m) => m.id === appState.mode)?.label ?? '');
  let modeVerb = $derived(modes.find((m) => m.id === appState.mode)?.verb ?? '');
  let statusText = $derived(appState.statusText);
  let showFormatOptions = $derived(appState.mode === 'compress');
  let showPassword = $derived(
    appState.mode === 'compress' ||
      appState.mode === 'decompress' ||
      appState.mode === 'encrypt' ||
      appState.mode === 'decrypt',
  );
  let passwordRequired = $derived(appState.mode === 'encrypt' || appState.mode === 'decrypt');
  let passwordPlaceholder = $derived(
    passwordRequired
      ? appState.mode === 'encrypt'
        ? '输入加密密码 (必填)...'
        : '输入解密密码 (必填)...'
      : '可选: 输入密码以加密压缩包...',
  );
  let passwordHint = $derived(
    passwordRequired
      ? appState.mode === 'encrypt'
        ? '加密后将使用此密码保护, 流式加密 (ChaCha20-Poly1305)'
        : '输入正确密码以解密文件'
      : '留空则不加密, 加密后兼容性降低',
  );

  let lvlRange = $derived(levelRange());
  let canExecute = $derived(canStart());

  // ===== 钥匙串菜单 =====
  let keychainMenuOpen = $state(false);

  function toggleKeychainMenu(): void {
    keychainMenuOpen = !keychainMenuOpen;
  }

  function closeKeychainMenu(): void {
    keychainMenuOpen = false;
  }

  async function onKeychainSave(): Promise<void> {
    closeKeychainMenu();
    await keychainSavePassword();
  }
  async function onKeychainLoad(): Promise<void> {
    closeKeychainMenu();
    await keychainLoadPassword();
  }
  async function onKeychainDelete(): Promise<void> {
    closeKeychainMenu();
    await keychainDeletePassword();
  }

  // 点击外部关闭钥匙串菜单
  let keychainContainer = $state<HTMLDivElement | null>(null);
  function onWindowClick(e: MouseEvent): void {
    if (keychainContainer && !keychainContainer.contains(e.target as Node)) {
      keychainMenuOpen = false;
    }
  }

  // ===== 日志自动滚动 =====
  let logContainer = $state<HTMLDivElement | null>(null);
  $effect(() => {
    // 依赖 logs 长度, 触发滚动
    const _len = appState.logs.length;
    if (logContainer) {
      // 下一帧滚动, 确保 DOM 已更新
      requestAnimationFrame(() => {
        if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
      });
    }
  });

  // ===== 日志颜色映射 =====
  const logColor: Record<LogEntry['kind'], string> = {
    info: 'text-text-dim',
    success: 'text-success',
    warn: 'text-warn',
    error: 'text-error',
  };
  const logIcon: Record<LogEntry['kind'], ComponentType> = {
    info: Info,
    success: CircleCheck,
    warn: TriangleAlert,
    error: CircleX,
  };

  // ===== 工作流执行 =====
  async function startTask(): Promise<void> {
    if (!canExecute) return;
    const mode = appState.mode;
    setWorking(true, '处理中...');
    resetProgress();
    pushLog(`开始${modeLabel}: ${appState.inputPath}`, 'info');
    try {
      if (mode === 'compress') {
        const exclude = appState.exclude.trim()
          ? appState.exclude.split(',').map((s) => s.trim()).filter(Boolean)
          : undefined;
        const result = await api.compress({
          input: appState.inputPath,
          output: appState.outputPath || undefined,
          format: appState.format,
          level: appState.level,
          password: appState.password || undefined,
          exclude,
          split: appState.splitSize.trim() || undefined,
        });
        pushLog(`压缩完成: ${result}`, 'success');
        showToast('压缩完成', 'success', result);
      } else if (mode === 'decompress') {
        const result = await api.decompress({
          input: appState.inputPath,
          output: appState.outputPath,
          password: appState.password || undefined,
        });
        pushLog(`解压完成: ${result}`, 'success');
        showToast('解压完成', 'success', result);
      } else if (mode === 'encrypt') {
        const out = appState.outputPath || `${appState.inputPath}.enc`;
        const result = await api.encrypt(appState.inputPath, out, appState.password);
        pushLog(`加密完成: ${result}`, 'success');
        showToast('加密完成', 'success', result);
      } else if (mode === 'decrypt') {
        const out = appState.outputPath || appState.inputPath.replace(/\.enc$/i, '');
        const result = await api.decrypt(appState.inputPath, out, appState.password);
        pushLog(`解密完成: ${result}`, 'success');
        showToast('解密完成', 'success', result);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      pushLog(`${modeLabel}失败: ${msg}`, 'error');
      showToast(`${modeLabel}失败`, 'error', msg);
    } finally {
      setWorking(false);
    }
  }

  // ===== 初始化: 主题 / 拖放 / 进度 =====
  $effect(() => {
    initTheme();
    window.addEventListener('click', onWindowClick);
    let stopDrag: (() => void) | null = null;
    let stopProgress: (() => void) | null = null;
    let cancelled = false;
    startDragDrop().then((fn) => {
      if (cancelled) fn();
      else stopDrag = fn;
    });
    startProgressListener(() => {
      // 进度完成回调 — 仅更新状态, 不重复 Toast (store 已处理)
      setWorking(false, '完成');
    }).then((fn) => {
      if (cancelled) fn();
      else stopProgress = fn;
    });
    return () => {
      cancelled = true;
      window.removeEventListener('click', onWindowClick);
      stopDrag?.();
      stopProgress?.();
    };
  });

  // ===== 拖放遮罩 =====
  let dragOver = $derived(appState.dragOver);

  // ===== 输出路径选择按钮 (依模式不同) =====
  let outputIsFolder = $derived(appState.mode === 'decompress');

  async function onPickOutput(): Promise<void> {
    if (outputIsFolder) await pickOutputFolder();
    else await pickOutputFile();
  }

  // ===== 格式变化时自动更新输出后缀 =====
  function onFormatChange(): void {
    autoFillOutput();
  }

  function onLevelChange(e: Event): void {
    const target = e.target as HTMLInputElement;
    appState.level = parseInt(target.value, 10) || 0;
  }

  // ===== 主题切换 =====
  function onToggleTheme(): void {
    toggleTheme();
  }

  function onOpenSettings(): void {
    showToast('设置面板 (即将推出)', 'info');
  }

  // ===== 拖放区域支持 (HTML5 拖放作为 Tauri 事件的后备) =====
  function onDragOver(e: DragEvent): void {
    e.preventDefault();
  }
  function onDrop(e: DragEvent): void {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      const path = (files[0] as unknown as { path?: string }).path ?? files[0].name;
      appState.inputPath = path;
      autoFillOutput();
      pushLog(`拖入文件: ${path}`, 'info');
    }
  }

  // ===== Toast 渲染 =====
  let currentToast = $derived(appState.toast);

  // ===== 状态栏颜色 =====
  let statusColor = $derived(
    appState.working ? 'bg-warn' : appState.statusText === '完成' ? 'bg-success' : 'bg-success',
  );
</script>

<div class="flex flex-col h-screen bg-bg text-text relative">
  <!-- Title Bar -->
  <TitleBar
    theme={appState.theme}
    onToggleTheme={onToggleTheme}
    onOpenSettings={onOpenSettings}
  />

  <!-- Brand + Mode Tabs -->
  <div
    class="flex items-center gap-3 px-4 py-2.5 border-b border-border/50 glass flex-shrink-0"
  >
    <div class="flex items-center gap-2 mr-4">
      <div class="w-7 h-7 rounded-mac-sm bg-accent/15 flex items-center justify-center">
        <Sparkles class="text-accent" size={15} />
      </div>
      <span class="font-bold text-base tracking-tight">SmartEx</span>
    </div>
    <div class="flex items-center gap-0.5 bg-bg-hover/60 rounded-mac p-0.5">
      {#each modes as m (m.id)}
        {@const Icon = m.icon}
        <button
          onclick={() => setMode(m.id)}
          disabled={appState.working}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-mac-sm text-sm font-medium transition-all duration-150 {appState.mode === m.id
            ? 'bg-accent text-white shadow-sm'
            : 'text-text-dim hover:text-text'} {appState.working
            ? 'opacity-50 cursor-not-allowed'
            : 'cursor-pointer'}"
        >
          <Icon size={15} />
          {m.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Main Content -->
  <div
    class="flex flex-1 gap-4 p-4 overflow-hidden min-h-0"
    role="main"
    ondragover={onDragOver}
    ondrop={onDrop}
  >
    <!-- Left: Parameters -->
    <div class="w-[380px] flex flex-col gap-4 overflow-y-auto flex-shrink-0 pr-1">
      <!-- 输入输出 -->
      <Panel title="输入 / 输出">
        <div class="flex flex-col gap-3">
          <div>
            <label for="input-path" class="text-sm font-medium text-text-dim mb-1.5 block">
              输入路径
            </label>
            <div class="flex gap-2">
              <input
                id="input-path"
                bind:value={appState.inputPath}
                oninput={() => autoFillOutput()}
                placeholder={appState.mode === 'decompress'
                  ? '选择归档文件...'
                  : appState.mode === 'decrypt'
                    ? '选择 .enc 文件...'
                    : '选择文件或文件夹...'}
                class="flex-1 min-w-0 px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm font-mono"
              />
              <Button variant="secondary" onclick={() => pickInputFile()} disabled={appState.working}>
                <FileInput size={16} />
              </Button>
            </div>
            <p class="text-xs text-text-dim/70 mt-1">支持拖放文件到窗口</p>
          </div>
          <div>
            <label for="output-path" class="text-sm font-medium text-text-dim mb-1.5 block">
              输出路径
              <span class="ml-1 text-text-dim/60 font-normal">(自动填充, 可修改)</span>
            </label>
            <div class="flex gap-2">
              <input
                id="output-path"
                bind:value={appState.outputPath}
                placeholder={outputIsFolder ? '选择输出目录...' : '选择输出文件...'}
                class="flex-1 min-w-0 px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm font-mono"
              />
              <Button variant="secondary" onclick={onPickOutput} disabled={appState.working}>
                {#if outputIsFolder}
                  <FolderOpen size={16} />
                {:else}
                  <Save size={16} />
                {/if}
              </Button>
            </div>
          </div>
        </div>
      </Panel>

      <!-- 选项 (依模式不同) -->
      <Panel title="选项">
        <div class="flex flex-col gap-4">
          {#if showFormatOptions}
            <!-- 压缩格式 -->
            <div>
              <label for="format-select" class="text-sm font-medium text-text-dim mb-1.5 block">
                压缩格式
              </label>
              <select
                id="format-select"
                bind:value={appState.format}
                onchange={onFormatChange}
                disabled={appState.working}
                class="w-full px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text focus:outline-none focus:border-accent transition-all text-sm appearance-none cursor-pointer"
              >
                {#each COMPRESS_FORMATS as fmt (fmt.value)}
                  <option value={fmt.value}>{fmt.label}</option>
                {/each}
              </select>
            </div>

            <!-- 压缩级别 -->
            <div>
              <label for="level-range" class="text-sm font-medium text-text-dim mb-1.5 flex justify-between">
                <span>压缩级别</span>
                <span class="font-mono text-accent">{appState.level}</span>
              </label>
              <input
                id="level-range"
                type="range"
                min={lvlRange.min}
                max={lvlRange.max}
                value={appState.level}
                oninput={onLevelChange}
                disabled={appState.working}
                class="w-full cursor-pointer"
              />
              <div class="flex justify-between text-xs text-text-dim mt-1">
                <span>存储 ({lvlRange.min})</span>
                <span>最高 ({lvlRange.max})</span>
              </div>
            </div>

            <!-- 排除规则 -->
            <div>
              <label for="exclude-input" class="text-sm font-medium text-text-dim mb-1.5 block">
                排除规则
                <span class="ml-1 text-text-dim/60 font-normal">(逗号分隔)</span>
              </label>
              <input
                id="exclude-input"
                bind:value={appState.exclude}
                disabled={appState.working}
                placeholder="*.DS_Store, *.tmp, node_modules/"
                class="w-full px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm font-mono"
              />
            </div>

            <!-- 分卷大小 -->
            <div>
              <label for="split-input" class="text-sm font-medium text-text-dim mb-1.5 block">
                分卷大小
                <span class="ml-1 text-text-dim/60 font-normal">(可选)</span>
              </label>
              <input
                id="split-input"
                bind:value={appState.splitSize}
                disabled={appState.working}
                placeholder="例如: 100M, 1G (留空不分卷)"
                class="w-full px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm font-mono"
              />
            </div>

            <!-- 智能推荐按钮 -->
            <button
              onclick={() => openContextWizard()}
              disabled={appState.working}
              class="flex items-center justify-center gap-2 px-3 py-2.5 rounded-mac-sm border border-accent/40 bg-accent/10 text-accent font-medium text-sm transition-all duration-150 hover:bg-accent/15 active:scale-[0.98] {appState.working
                ? 'opacity-50 cursor-not-allowed'
                : 'cursor-pointer'}"
            >
              <Sparkles size={16} />
              智能推荐格式
            </button>
          {/if}

          <!-- 密码 (压缩/解压可选, 加密/解密必填) -->
          {#if showPassword}
            <div>
              <label for="password-input" class="text-sm font-medium text-text-dim mb-1.5 flex justify-between items-center">
                <span>
                  密码
                  {#if passwordRequired}
                    <span class="text-error ml-0.5">*</span>
                  {/if}
                </span>
                <!-- 钥匙串菜单 -->
                <div class="relative" bind:this={keychainContainer}>
                  <button
                    onclick={toggleKeychainMenu}
                    disabled={appState.working}
                    title="钥匙串"
                    class="flex items-center gap-1 px-2 py-0.5 rounded-mac-sm text-xs text-text-dim hover:text-accent hover:bg-bg-hover transition-all {appState.working
                      ? 'opacity-50 cursor-not-allowed'
                      : 'cursor-pointer'}"
                  >
                    <KeyRound size={13} />
                    钥匙串
                    <ChevronDown size={12} class="transition-transform {keychainMenuOpen ? 'rotate-180' : ''}" />
                  </button>
                  {#if keychainMenuOpen}
                    <div class="absolute right-0 top-full mt-1 z-30 min-w-[160px] glass rounded-mac-sm border border-border/60 shadow-xl py-1 animate-scale-in">
                      <button
                        onclick={onKeychainSave}
                        class="w-full flex items-center gap-2 px-3 py-1.5 text-sm text-text hover:bg-bg-hover transition-colors text-left"
                      >
                        <Save size={14} class="text-accent" />
                        保存密码
                      </button>
                      <button
                        onclick={onKeychainLoad}
                        class="w-full flex items-center gap-2 px-3 py-1.5 text-sm text-text hover:bg-bg-hover transition-colors text-left"
                      >
                        <KeyRound size={14} class="text-success" />
                        读取密码
                      </button>
                      <div class="h-px bg-border/50 my-1"></div>
                      <button
                        onclick={onKeychainDelete}
                        class="w-full flex items-center gap-2 px-3 py-1.5 text-sm text-error hover:bg-bg-hover transition-colors text-left"
                      >
                        <Trash2 size={14} />
                        删除密码
                      </button>
                    </div>
                  {/if}
                </div>
              </label>
              <div class="relative">
                <input
                  id="password-input"
                  type={appState.showPassword ? 'text' : 'password'}
                  bind:value={appState.password}
                  disabled={appState.working}
                  placeholder={passwordPlaceholder}
                  class="w-full px-3 py-2 pr-9 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all text-sm font-mono"
                />
                <button
                  onclick={() => (appState.showPassword = !appState.showPassword)}
                  type="button"
                  title={appState.showPassword ? '隐藏密码' : '显示密码'}
                  class="absolute right-2 top-1/2 -translate-y-1/2 text-text-dim hover:text-text transition-colors p-1"
                >
                  {#if appState.showPassword}
                    <EyeOff size={15} />
                  {:else}
                    <Eye size={15} />
                  {/if}
                </button>
              </div>
              <p class="text-xs text-text-dim mt-1">{passwordHint}</p>
            </div>
          {/if}

          {#if appState.mode === 'compress' && !appState.password}
            <div class="text-xs text-text-dim bg-bg-hover/50 rounded-mac-sm px-3 py-2 leading-relaxed">
              提示: 不确定用什么格式? 点击「智能推荐」让 SmartEx 根据场景自动选择。
            </div>
          {/if}
        </div>
      </Panel>

      <!-- 开始按钮 -->
      <Button
        variant="primary"
        onclick={startTask}
        disabled={!canExecute}
        loading={appState.working}
        class="w-full"
      >
        {modeVerb}
      </Button>
      {#if !canExecute && !appState.working}
        <p class="text-xs text-text-dim/70 text-center -mt-2">
          {#if !appState.inputPath}
            请选择输入文件
          {:else if passwordRequired && !appState.password}
            请输入密码
          {:else if appState.mode === 'decompress' && !appState.outputPath}
            请选择输出目录
          {/if}
        </p>
      {/if}
    </div>

    <!-- Right: Progress + Log -->
    <div class="flex-1 flex flex-col gap-4 overflow-hidden min-w-0">
      <Panel title="进度">
        <ProgressBar
          progress={appState.progress}
          bytesDone={appState.bytesDone}
          bytesTotal={appState.bytesTotal}
          indeterminate={appState.working && appState.progress === 0}
          message={appState.working ? appState.statusText : appState.progress >= 100 ? '完成' : '空闲'}
        />
        {#if appState.working}
          <div class="mt-3 flex justify-end">
            <Button variant="danger" onclick={() => {
              pushLog('取消请求已发送 (后端将在下一个检查点停止)', 'warn');
              showToast('已请求取消', 'warn');
            }}>
              取消
            </Button>
          </div>
        {/if}
      </Panel>

      <Panel title="日志" class="flex-1 min-h-0">
        <div class="flex justify-end mb-2">
          <button
            onclick={clearLogs}
            class="flex items-center gap-1 text-xs text-text-dim hover:text-text transition-colors px-2 py-1 rounded-mac-sm hover:bg-bg-hover"
          >
            <Eraser size={12} />
            清空
          </button>
        </div>
        <div
          bind:this={logContainer}
          class="h-[calc(100%-2.5rem)] overflow-y-auto font-mono text-xs space-y-1 pr-1"
        >
          {#if appState.logs.length === 0}
            <p class="text-text-dim italic">暂无日志</p>
          {:else}
            {#each appState.logs as log (log.time + log.text + Math.random())}
              {@const Icon = logIcon[log.kind]}
              <div class="flex gap-2 leading-relaxed items-start">
                <span class="text-text-dim/60 flex-shrink-0">{log.time}</span>
                <Icon size={12} class={`flex-shrink-0 mt-0.5 ${logColor[log.kind]}`} />
                <span class={`flex-shrink-0 font-semibold ${logColor[log.kind]}`}>
                  [{log.kind.toUpperCase()}]
                </span>
                <span class="text-text break-all">{log.text}</span>
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
        <span class={`w-2 h-2 rounded-full ${statusColor} ${appState.working ? 'animate-pulse' : ''}`}></span>
        {statusText}
      </span>
      <span>模式: {modeLabel}</span>
      {#if appState.inputPath}
        <span class="truncate max-w-[300px]">输入: {appState.inputPath}</span>
      {/if}
    </div>
    <div class="flex items-center gap-4">
      <span>SmartEx v0.6.0</span>
    </div>
  </div>

  <!-- 拖放遮罩 -->
  {#if dragOver}
    <div
      class="fixed inset-0 z-40 flex items-center justify-center pointer-events-none bg-accent/10 backdrop-blur-sm"
    >
      <div
        class="flex flex-col items-center gap-3 p-8 rounded-mac-lg border-2 border-dashed border-accent bg-bg-panel/80 animate-scale-in"
      >
        <Upload class="text-accent" size={48} />
        <p class="text-lg font-semibold text-text">释放以添加文件</p>
        <p class="text-sm text-text-dim">支持任意文件或文件夹</p>
      </div>
    </div>
  {/if}

  <!-- Toast -->
  {#if currentToast}
    {#key currentToast.id}
      <div class="fixed bottom-12 right-6 z-50">
        <Toast
          type={currentToast.kind}
          message={currentToast.message}
          description={currentToast.description}
          duration={currentToast.duration ?? 3500}
          ondismiss={dismissToast}
        />
      </div>
    {/key}
  {/if}

  <!-- 智能推荐向导 -->
  <ContextWizard />
</div>
