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
    Workflow,
    Play,
    ListTree,
    Check,
    X,
    FileText,
    Folder,
    Loader2,
  } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import Panel from '$lib/components/Panel.svelte';
  import Button from '$lib/components/Button.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import ContextWizard from '$lib/components/ContextWizard.svelte';
  import SettingsPanel from '$lib/components/SettingsPanel.svelte';
  import WorkflowEditor from '$lib/components/WorkflowEditor.svelte';
  import BackgroundLayer from '$lib/components/BackgroundLayer.svelte';
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
    pickInputFolder,
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
    browseArchive,
    toggleFileSelection,
    selectAllFiles,
    clearSelection,
    extractSelected,
    type Mode,
    type LogEntry,
  } from '$lib/stores/app.svelte';
  import {
    settings,
    settingsUI,
    openSettings,
    applyAllAppearance,
    initSystemThemeListener,
    matchShortcut,
  } from '$lib/stores/settings.svelte';
  import {
    workflowsState,
    openWorkflowEditor,
    runWorkflowFromMain,
  } from '$lib/stores/workflows.svelte';
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

  // ===== L2: 模式过滤 (关闭的模式从 tabs 消失) =====
  let visibleModes = $derived(
    modes.filter((m) => settings.enabledModes[m.id]),
  );

  // 确保当前模式始终启用, 否则切到第一个启用模式
  $effect(() => {
    if (!settings.enabledModes[appState.mode] && visibleModes.length > 0) {
      setMode(visibleModes[0].id);
    }
  });

  // ===== L1: 布局派生 =====
  let layoutClass = $derived(
    settings.layout === 'left-right'
      ? 'flex flex-row'
      : settings.layout === 'right-left'
        ? 'flex flex-row-reverse'
        : 'flex flex-col',
  );
  let paramPanelClass = $derived(
    settings.layout === 'top-bottom' ? 'w-full' : 'w-[380px] flex-shrink-0',
  );
  let logPanelClass = $derived(
    settings.layout === 'top-bottom' ? 'w-full' : 'flex-1 min-w-0',
  );

  // ===== L2: 功能开关派生 =====
  let showContextWizardBtn = $derived(settings.enabledFeatures.contextWizard);
  let showKeychain = $derived(settings.enabledFeatures.keychain);
  let showSplitSize = $derived(settings.enabledFeatures.splitSize);
  let showExclude = $derived(settings.enabledFeatures.exclude);

  // ===== L3: 已保存工作流 =====
  let savedWorkflows = $derived(workflowsState.workflows);

  async function onRunWorkflow(id: string): Promise<void> {
    await runWorkflowFromMain(id);
  }

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
          conflictPolicy: appState.conflictPolicy,
          preserveSymlinks: appState.preserveSymlinks,
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

  // ===== 初始化: 主题 / 拖放 / 进度 / 自定义系统 =====
  $effect(() => {
    initTheme();
    // L1: 应用自定义外观 (主题色 / 字体 / 主题)
    applyAllAppearance();
    let stopSystemTheme = initSystemThemeListener();
    // L1: 全局快捷键
    window.addEventListener('keydown', onGlobalKeydown);
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
      window.removeEventListener('keydown', onGlobalKeydown);
      window.removeEventListener('click', onWindowClick);
      stopSystemTheme();
      stopDrag?.();
      stopProgress?.();
    };
  });

  // L1: 主题/字体/背景/动效变化时重新应用
  $effect(() => {
    void settings.theme;
    void settings.accentColor;
    void settings.fontFamily;
    void settings.fontSize;
    void settings.background.type;
    void settings.panelOpacity;
    void settings.blurStrength;
    void settings.motion.enabled;
    applyAllAppearance();
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
    openSettings();
  }

  function onOpenWorkflowEditor(): void {
    openWorkflowEditor();
  }

  // ===== L1: 全局快捷键 =====
  function onGlobalKeydown(e: KeyboardEvent): void {
    // 设置面板打开时不触发全局快捷键 (避免与快捷键录制冲突)
    if (settingsUI.open) return;
    // 工作流编辑器打开时也不触发
    if (workflowsState.editorOpen) return;
    // 开始执行
    if (matchShortcut(settings.shortcuts.start, e)) {
      e.preventDefault();
      if (canStart() && !appState.working) {
        void startTask();
      }
      return;
    }
    // 取消
    if (matchShortcut(settings.shortcuts.cancel, e)) {
      if (appState.working) {
        e.preventDefault();
        void onCancelTask();
      }
      return;
    }
    // 清日志
    if (matchShortcut(settings.shortcuts.clearLogs, e)) {
      e.preventDefault();
      clearLogs();
      return;
    }
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
    appState.working
      ? 'bg-warn'
      : appState.statusText.includes('失败') || appState.statusText.includes('取消')
        ? 'bg-error'
        : 'bg-success',
  );

  // ===== 解压选项派生 =====
  let isDecompressMode = $derived(appState.mode === 'decompress');
  let showArchiveBrowser = $derived(appState.showArchiveBrowser);
  let archiveEntries = $derived(appState.archiveEntries);
  let selectedCount = $derived(appState.selectedFiles.size);

  // 归档浏览加载状态
  let browsingArchive = $state(false);

  async function onBrowseArchive(): Promise<void> {
    if (!appState.inputPath) {
      showToast('请先选择归档文件', 'warn');
      return;
    }
    browsingArchive = true;
    try {
      await browseArchive();
    } finally {
      browsingArchive = false;
    }
  }

  async function onExtractSelected(): Promise<void> {
    await extractSelected();
  }

  function onCloseArchiveBrowser(): void {
    appState.showArchiveBrowser = false;
  }

  // ===== Bug 3 修复: 取消任务真正调用后端 =====
  async function onCancelTask(): Promise<void> {
    try {
      await api.cancelTask();
      pushLog('取消请求已发送 (后端将在下一个检查点停止)', 'warn');
      showToast('已请求取消', 'warn');
    } catch (e) {
      pushLog(`取消失败: ${String(e)}`, 'error');
      showToast('取消失败', 'error', String(e));
    }
  }

  // ===== 文件大小格式化 =====
  function formatBytes(n: number): string {
    if (n >= 1024 * 1024 * 1024) return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`;
    if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${n} B`;
  }
</script>

<div class="flex flex-col h-screen bg-bg text-text relative">
  <BackgroundLayer />
  <!-- Title Bar -->
  <TitleBar
    theme={appState.theme}
    onToggleTheme={onToggleTheme}
    onOpenSettings={onOpenSettings}
    onOpenWorkflowEditor={onOpenWorkflowEditor}
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
      {#each visibleModes as m (m.id)}
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

    <!-- L3: 已保存工作流按钮 -->
    {#if savedWorkflows.length > 0}
      <div class="flex items-center gap-1 ml-auto">
        <Workflow size={14} class="text-text-dim" />
        {#each savedWorkflows.slice(0, 4) as wf (wf.id)}
          <button
            onclick={() => onRunWorkflow(wf.id)}
            disabled={appState.working}
            title={wf.name}
            class="flex items-center gap-1 px-2 py-1 rounded-mac-sm text-xs font-medium transition-all {appState.working
              ? 'opacity-50 cursor-not-allowed text-text-dim'
              : 'text-text-dim hover:text-accent hover:bg-bg-hover cursor-pointer'}"
          >
            <Play size={11} />
            {wf.name}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Main Content -->
  <div
    class="{layoutClass} flex-1 gap-4 p-4 overflow-hidden min-h-0"
    role="main"
    ondragover={onDragOver}
    ondrop={onDrop}
  >
    <!-- Left: Parameters -->
    <div class="{paramPanelClass} flex flex-col gap-4 overflow-y-auto pr-1">
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
              <Button variant="secondary" onclick={() => pickInputFile()} disabled={appState.working} title="选择文件">
                <FileInput size={16} />
              </Button>
              {#if appState.mode === 'compress' || appState.mode === 'encrypt'}
                <Button variant="secondary" onclick={() => pickInputFolder()} disabled={appState.working} title="选择文件夹">
                  <FolderOpen size={16} />
                </Button>
              {/if}
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
            {#if showExclude}
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
            {/if}

            <!-- 分卷大小 -->
            {#if showSplitSize}
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
            {/if}

            <!-- 智能推荐按钮 -->
            {#if showContextWizardBtn}
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
          {/if}

          <!-- 解压选项 -->
          {#if isDecompressMode}
            <!-- 冲突策略 -->
            <div>
              <label class="text-sm font-medium text-text-dim mb-1.5 block">冲突处理策略</label>
              <div class="grid grid-cols-3 gap-1.5">
                <button
                  type="button"
                  onclick={() => (appState.conflictPolicy = 'overwrite')}
                  disabled={appState.working}
                  class="px-2 py-1.5 rounded-mac-sm text-xs font-medium transition-all {appState.conflictPolicy === 'overwrite'
                    ? 'bg-accent text-white shadow-sm'
                    : 'bg-bg-hover text-text-dim hover:text-text'}"
                >
                  覆盖
                </button>
                <button
                  type="button"
                  onclick={() => (appState.conflictPolicy = 'skip')}
                  disabled={appState.working}
                  class="px-2 py-1.5 rounded-mac-sm text-xs font-medium transition-all {appState.conflictPolicy === 'skip'
                    ? 'bg-accent text-white shadow-sm'
                    : 'bg-bg-hover text-text-dim hover:text-text'}"
                >
                  跳过
                </button>
                <button
                  type="button"
                  onclick={() => (appState.conflictPolicy = 'rename')}
                  disabled={appState.working}
                  class="px-2 py-1.5 rounded-mac-sm text-xs font-medium transition-all {appState.conflictPolicy === 'rename'
                    ? 'bg-accent text-white shadow-sm'
                    : 'bg-bg-hover text-text-dim hover:text-text'}"
                >
                  重命名
                </button>
              </div>
              <p class="text-xs text-text-dim/70 mt-1">
                {#if appState.conflictPolicy === 'overwrite'}
                  同名文件将被覆盖
                {:else if appState.conflictPolicy === 'skip'}
                  同名文件将保留原版
                {:else}
                  同名文件自动重命名 (如 file_1.txt)
                {/if}
              </p>
            </div>

            <!-- 保留符号链接 -->
            <div class="flex items-center justify-between">
              <div>
                <span class="text-sm font-medium text-text block">保留符号链接</span>
                <span class="text-xs text-text-dim/70">仅 tar 系列, Windows 自动跳过</span>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={appState.preserveSymlinks}
                onclick={() => (appState.preserveSymlinks = !appState.preserveSymlinks)}
                disabled={appState.working}
                class="relative w-10 h-6 rounded-full transition-colors {appState.preserveSymlinks
                  ? 'bg-accent'
                  : 'bg-bg-hover'} {appState.working ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}"
              >
                <span
                  class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform {appState.preserveSymlinks
                    ? 'translate-x-4'
                    : ''}"
                ></span>
              </button>
            </div>

            <!-- 浏览归档 -->
            <button
              onclick={onBrowseArchive}
              disabled={appState.working || !appState.inputPath || browsingArchive}
              class="flex items-center justify-center gap-2 px-3 py-2.5 rounded-mac-sm border border-accent/40 bg-accent/10 text-accent font-medium text-sm transition-all duration-150 hover:bg-accent/15 active:scale-[0.98] {appState.working || !appState.inputPath || browsingArchive
                ? 'opacity-50 cursor-not-allowed'
                : 'cursor-pointer'}"
            >
              {#if browsingArchive}
                <Loader2 size={16} class="animate-spin" />
                读取中...
              {:else}
                <ListTree size={16} />
                浏览归档 (部分解压)
              {/if}
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
                {#if showKeychain}
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
                {/if}
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
    <div class="{logPanelClass} flex flex-col gap-4 overflow-hidden">
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
            <Button variant="danger" onclick={onCancelTask}>
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
            {#each appState.logs as log (log.id)}
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
      <div class="fixed bottom-12 right-6 z-[60]">
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

  <!-- 归档浏览面板 (部分解压) -->
  {#if showArchiveBrowser}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div class="w-full max-w-2xl max-h-[80vh] flex flex-col bg-bg-panel rounded-mac-lg border border-border/60 shadow-2xl animate-scale-in">
        <!-- 头部 -->
        <div class="flex items-center justify-between px-4 py-3 border-b border-border/50">
          <div class="flex items-center gap-2">
            <ListTree size={16} class="text-accent" />
            <h2 class="text-sm font-semibold text-text">归档内容浏览</h2>
            <span class="text-xs text-text-dim">({archiveEntries.length} 项)</span>
          </div>
          <button
            onclick={onCloseArchiveBrowser}
            class="p-1 rounded-mac-sm text-text-dim hover:text-text hover:bg-bg-hover transition-colors"
            aria-label="关闭"
          >
            <X size={16} />
          </button>
        </div>

        <!-- 工具栏 -->
        <div class="flex items-center justify-between px-4 py-2 border-b border-border/50 bg-bg-hover/30">
          <div class="flex items-center gap-2">
            <button
              onclick={selectAllFiles}
              class="flex items-center gap-1 px-2 py-1 text-xs text-text-dim hover:text-accent hover:bg-bg-hover rounded-mac-sm transition-colors"
            >
              <Check size={12} />
              全选
            </button>
            <button
              onclick={clearSelection}
              class="flex items-center gap-1 px-2 py-1 text-xs text-text-dim hover:text-text hover:bg-bg-hover rounded-mac-sm transition-colors"
            >
              <X size={12} />
              清空
            </button>
          </div>
          <span class="text-xs text-text-dim">
            已选 <span class="text-accent font-medium">{selectedCount}</span> 个文件
          </span>
        </div>

        <!-- 文件列表 -->
        <div class="flex-1 overflow-y-auto p-2 min-h-0">
          {#if archiveEntries.length === 0}
            <div class="flex flex-col items-center justify-center py-12 text-text-dim">
              <ListTree size={32} class="opacity-40 mb-2" />
              <p class="text-sm">归档为空或无法读取</p>
            </div>
          {:else}
            {#each archiveEntries as entry, i (entry.path)}
              {@const selected = appState.selectedFiles.has(entry.path)}
              <button
                type="button"
                onclick={() => toggleFileSelection(entry.path)}
                disabled={entry.is_dir}
                style="animation-delay: {Math.min(i * 18, 360)}ms;"
                class="w-full flex items-center gap-3 px-3 py-2 rounded-mac-sm text-left transition-colors stagger-item {selected
                  ? 'bg-accent/15 text-text'
                  : entry.is_dir
                    ? 'text-text-dim cursor-not-allowed'
                    : 'text-text hover:bg-bg-hover'}"
              >
                <!-- 勾选框 -->
                <span
                  class="flex-shrink-0 w-4 h-4 rounded border flex items-center justify-center transition-all {selected
                    ? 'bg-accent border-accent'
                    : entry.is_dir
                      ? 'border-border/40 opacity-40'
                      : 'border-border'}"
                >
                  {#if selected}
                    <Check size={11} class="text-white" />
                  {/if}
                </span>

                <!-- 图标 -->
                {#if entry.is_dir}
                  <Folder size={14} class="flex-shrink-0 text-warn" />
                {:else}
                  <FileText size={14} class="flex-shrink-0 text-text-dim" />
                {/if}

                <!-- 路径 -->
                <span class="flex-1 min-w-0 truncate text-sm font-mono">{entry.path}</span>

                <!-- 大小 -->
                {#if !entry.is_dir}
                  <span class="flex-shrink-0 text-xs text-text-dim font-mono">
                    {formatBytes(entry.size)}
                  </span>
                {/if}
              </button>
            {/each}
          {/if}
        </div>

        <!-- 底部操作 -->
        <div class="flex items-center justify-between gap-3 px-4 py-3 border-t border-border/50">
          <p class="text-xs text-text-dim">
            输出目录: <span class="font-mono">{appState.outputPath || '(未设置)'}</span>
          </p>
          <div class="flex items-center gap-2">
            <Button variant="secondary" onclick={onCloseArchiveBrowser}>取消</Button>
            <Button
              variant="primary"
              onclick={onExtractSelected}
              disabled={selectedCount === 0 || !appState.outputPath || appState.working}
              loading={appState.working}
            >
              解压选中 ({selectedCount})
            </Button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- 智能推荐向导 -->
  <ContextWizard />

  <!-- L1+L2: 设置面板 -->
  <SettingsPanel />

  <!-- L3: 工作流编辑器 -->
  <WorkflowEditor />
</div>
