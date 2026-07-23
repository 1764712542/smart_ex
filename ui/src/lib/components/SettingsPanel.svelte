<script lang="ts">
  // 自定义系统 L1 (外观+布局) + L2 (功能模块化) 设置面板
  // 右侧滑出, Mac 风格分组
  import {
    X,
    Palette,
    Sun,
    Moon,
    Monitor,
    Type,
    LayoutGrid,
    Keyboard,
    Download,
    Upload,
    RotateCcw,
    Archive,
    ArchiveRestore,
    Lock,
    LockOpen,
    Sparkles,
    KeyRound,
    Split,
    FilterX,
    List,
    ShieldOff,
    Power,
  } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import {
    settings,
    settingsUI,
    closeSettings,
    PRESET_ACCENTS,
    FONT_OPTIONS,
    FONT_SIZE_PX,
    LAYOUT_OPTIONS,
    DEFAULT_SHORTCUTS,
    setAccent,
    setTheme,
    setFontFamily,
    setFontSize,
    setLayout,
    setShortcut,
    resetShortcuts,
    toggleMode,
    toggleFeature,
    exportProfileToFile,
    importProfileFromFile,
    resetToDefault,
    formatShortcut,
    type Theme,
    type FontSize,
    type Layout,
    type Shortcuts,
    type EnabledModes,
    type EnabledFeatures,
  } from '$lib/stores/settings.svelte';

  // ===== 快捷键录制 =====
  let recordingKey: keyof Shortcuts | null = $state(null);

  function startRecording(key: keyof Shortcuts): void {
    recordingKey = key;
  }

  function stopRecording(): void {
    recordingKey = null;
  }

  function onKeydownCapture(e: KeyboardEvent): void {
    if (!recordingKey) return;
    e.preventDefault();
    e.stopPropagation();
    // 忽略纯修饰键
    if (['Control', 'Meta', 'Alt', 'Shift'].includes(e.key)) return;
    const sc = formatShortcut(e);
    if (sc) {
      setShortcut(recordingKey, sc);
      stopRecording();
    }
  }

  // 点击外部关闭录制
  $effect(() => {
    if (!recordingKey) return;
    const handler = () => stopRecording();
    window.addEventListener('click', handler);
    return () => window.removeEventListener('click', handler);
  });

  // ===== 主题选项 =====
  const themeOptions: { value: Theme; label: string; icon: ComponentType }[] = [
    { value: 'dark', label: '深色', icon: Moon },
    { value: 'light', label: '浅色', icon: Sun },
    { value: 'system', label: '跟随系统', icon: Monitor },
  ];

  const fontSizeOptions: { value: FontSize; label: string }[] = [
    { value: 'small', label: '小' },
    { value: 'medium', label: '中' },
    { value: 'large', label: '大' },
  ];

  // ===== L2 模式定义 =====
  const modeRows: { key: keyof EnabledModes; label: string; icon: ComponentType }[] = [
    { key: 'compress', label: '压缩', icon: Archive },
    { key: 'decompress', label: '解压', icon: ArchiveRestore },
    { key: 'encrypt', label: '加密', icon: Lock },
    { key: 'decrypt', label: '解密', icon: LockOpen },
  ];

  const featureRows: { key: keyof EnabledFeatures; label: string; desc: string; icon: ComponentType }[] = [
    { key: 'contextWizard', label: '智能推荐', desc: '上下文感知格式向导', icon: Sparkles },
    { key: 'keychain', label: '钥匙串', desc: '密码保存到系统钥匙串', icon: KeyRound },
    { key: 'splitSize', label: '分卷压缩', desc: '按大小分卷归档', icon: Split },
    { key: 'exclude', label: '文件排除', desc: '压缩时排除指定文件', icon: FilterX },
    { key: 'archiveList', label: '归档浏览', desc: '查看归档内容列表', icon: List },
    { key: 'secureDelete', label: '安全删除', desc: '删除源文件防恢复', icon: ShieldOff },
  ];

  // ===== 快捷键项 =====
  const shortcutItems: { key: keyof Shortcuts; label: string }[] = [
    { key: 'start', label: '开始执行' },
    { key: 'cancel', label: '取消任务' },
    { key: 'clearLogs', label: '清空日志' },
  ];

  let open = $derived(settingsUI.open);
</script>

<svelte:window onkeydown={onKeydownCapture} />

{#snippet ToggleSwitch(on: boolean, onclick: () => void)}
  <button
    {onclick}
    role="switch"
    aria-checked={on}
    class="relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 {on
      ? 'bg-accent'
      : 'bg-bg-hover border border-border'}"
  >
    <span
      class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transition-transform duration-200 {on
        ? 'translate-x-4'
        : 'translate-x-0'}"
    ></span>
  </button>
{/snippet}

<!-- 遮罩 -->
{#if open}
  <div
    class="fixed inset-0 z-40 bg-black/30 backdrop-blur-[2px] animate-fade-in"
    role="button"
    tabindex="-1"
    aria-label="关闭设置"
    onclick={closeSettings}
    onkeydown={(e) => { if (e.key === 'Escape') closeSettings(); }}
  ></div>
{/if}

<!-- 面板 -->
<div
  class="fixed top-0 right-0 bottom-0 z-50 w-[400px] max-w-[90vw] glass border-l border-border/60 shadow-2xl flex flex-col transition-transform duration-300 ease-out {open
    ? 'translate-x-0'
    : 'translate-x-full'}"
  role="dialog"
  aria-modal="true"
  aria-label="设置"
>
  <!-- 头部 -->
  <div class="flex items-center justify-between px-5 py-4 border-b border-border/50 flex-shrink-0">
    <div class="flex items-center gap-2.5">
      <div class="w-7 h-7 rounded-mac-sm bg-accent/15 flex items-center justify-center">
        <Palette class="text-accent" size={16} />
      </div>
      <h2 class="text-base font-semibold text-text">设置</h2>
    </div>
    <button
      onclick={closeSettings}
      class="p-1.5 rounded-mac-sm text-text-dim hover:text-text hover:bg-bg-hover transition-all"
      aria-label="关闭"
    >
      <X size={18} />
    </button>
  </div>

  <!-- 滚动内容 -->
  <div class="flex-1 overflow-y-auto px-5 py-4 space-y-6">

    <!-- ===== L1-1: 主题色 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Palette size={13} />
        主题色
      </h3>
      <div class="flex items-center gap-2.5 flex-wrap">
        {#each PRESET_ACCENTS as preset (preset.name)}
          <button
            onclick={() => setAccent(preset.color)}
            title={preset.name}
            class="w-7 h-7 rounded-full transition-all duration-150 hover:scale-110 active:scale-95 {settings.accentColor.toLowerCase() === preset.color.toLowerCase()
              ? 'ring-2 ring-offset-2 ring-offset-bg-panel ring-text'
              : ''}"
            style="background: {preset.color};"
            aria-label="主题色 {preset.name}"
          ></button>
        {/each}
        <div class="relative w-7 h-7 rounded-full overflow-hidden border border-border/60">
          <input
            type="color"
            value={settings.accentColor}
            oninput={(e) => setAccent(e.currentTarget.value)}
            class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
            title="自定义颜色"
            aria-label="自定义颜色"
          />
        </div>
      </div>
      <p class="text-xs text-text-dim/70 mt-2 font-mono">当前: {settings.accentColor}</p>
    </section>

    <!-- ===== L1-2: 明暗主题 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Sun size={13} />
        明暗主题
      </h3>
      <div class="grid grid-cols-3 gap-2">
        {#each themeOptions as opt (opt.value)}
          {@const Icon = opt.icon}
          <button
            onclick={() => setTheme(opt.value)}
            class="flex flex-col items-center gap-1.5 py-2.5 rounded-mac-sm border text-xs font-medium transition-all duration-150 {settings.theme === opt.value
              ? 'border-accent bg-accent/10 text-accent'
              : 'border-border bg-bg-hover/40 text-text-dim hover:bg-bg-hover'}"
          >
            <Icon size={16} />
            {opt.label}
          </button>
        {/each}
      </div>
    </section>

    <!-- ===== L1-3: 字体 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Type size={13} />
        字体
      </h3>
      <div class="space-y-3">
        <div>
          <label for="font-family" class="text-xs text-text-dim mb-1.5 block">字体族</label>
          <select
            id="font-family"
            value={settings.fontFamily}
            onchange={(e) => setFontFamily(e.currentTarget.value)}
            class="w-full px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text focus:outline-none focus:border-accent transition-all text-sm appearance-none cursor-pointer"
          >
            {#each FONT_OPTIONS as font (font.value)}
              <option value={font.value}>{font.label}</option>
            {/each}
          </select>
        </div>
        <div>
          <label class="text-xs text-text-dim mb-1.5 block">字号</label>
          <div class="grid grid-cols-3 gap-2">
            {#each fontSizeOptions as opt (opt.value)}
              <button
                onclick={() => setFontSize(opt.value)}
                class="py-1.5 rounded-mac-sm border text-xs font-medium transition-all {settings.fontSize === opt.value
                  ? 'border-accent bg-accent/10 text-accent'
                  : 'border-border bg-bg-hover/40 text-text-dim hover:bg-bg-hover'}"
                style="font-size: {FONT_SIZE_PX[opt.value]};"
              >
                {opt.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
    </section>

    <!-- ===== L1-4: 面板布局 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <LayoutGrid size={13} />
        面板布局
      </h3>
      <div class="space-y-1.5">
        {#each LAYOUT_OPTIONS as opt (opt.value)}
          <button
            onclick={() => setLayout(opt.value)}
            class="w-full flex items-center justify-between px-3 py-2 rounded-mac-sm border text-left transition-all {settings.layout === opt.value
              ? 'border-accent bg-accent/10'
              : 'border-border bg-bg-hover/40 hover:bg-bg-hover'}"
          >
            <div>
              <div class="text-sm font-medium text-text">{opt.label}</div>
              <div class="text-xs text-text-dim">{opt.desc}</div>
            </div>
            <div class="flex items-center gap-1">
              {#if settings.layout === opt.value}
                <div class="w-2 h-2 rounded-full bg-accent"></div>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </section>

    <!-- ===== L1-5: 快捷键 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Keyboard size={13} />
        快捷键
        <button
          onclick={resetShortcuts}
          class="ml-auto text-[11px] text-text-dim hover:text-accent transition-colors flex items-center gap-0.5"
        >
          <RotateCcw size={11} />
          重置
        </button>
      </h3>
      <div class="space-y-1.5">
        {#each shortcutItems as item (item.key)}
          <div class="flex items-center justify-between px-3 py-2 rounded-mac-sm bg-bg-hover/40 border border-border">
            <span class="text-sm text-text">{item.label}</span>
            <button
              onclick={(e) => { e.stopPropagation(); startRecording(item.key); }}
              class="px-2.5 py-1 rounded-mac-sm text-xs font-mono border transition-all {recordingKey === item.key
                ? 'border-accent bg-accent/15 text-accent animate-pulse'
                : 'border-border bg-bg-panel text-text-dim hover:text-text hover:border-text-dim'}"
            >
              {recordingKey === item.key ? '按下组合键...' : settings.shortcuts[item.key]}
            </button>
          </div>
        {/each}
      </div>
    </section>

    <!-- ===== L2-1: 模式启停 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Power size={13} />
        模式启停
        <span class="ml-auto text-[11px] text-text-dim/70 font-normal">至少保留一个</span>
      </h3>
      <div class="space-y-1.5">
        {#each modeRows as row (row.key)}
          {@const Icon = row.icon}
          <div class="flex items-center justify-between px-3 py-2 rounded-mac-sm bg-bg-hover/40 border border-border">
            <div class="flex items-center gap-2.5">
              <Icon size={15} class={settings.enabledModes[row.key] ? 'text-accent' : 'text-text-dim'} />
              <span class="text-sm text-text">{row.label}</span>
            </div>
            <button
              onclick={() => toggleMode(row.key)}
              role="switch"
              aria-checked={settings.enabledModes[row.key]}
              class="relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 {settings.enabledModes[row.key]
                ? 'bg-accent'
                : 'bg-bg-hover border border-border'}"
            >
              <span
                class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transition-transform duration-200 {settings.enabledModes[row.key]
                  ? 'translate-x-4'
                  : 'translate-x-0'}"
              ></span>
            </button>
          </div>
        {/each}
      </div>
    </section>

    <!-- ===== L2-2: 功能启停 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Sparkles size={13} />
        功能启停
      </h3>
      <div class="space-y-1.5">
        {#each featureRows as row (row.key)}
          {@const Icon = row.icon}
          <div class="flex items-center justify-between px-3 py-2 rounded-mac-sm bg-bg-hover/40 border border-border">
            <div class="flex items-center gap-2.5 min-w-0">
              <Icon size={15} class={settings.enabledFeatures[row.key] ? 'text-accent' : 'text-text-dim'} />
              <div class="min-w-0">
                <div class="text-sm text-text">{row.label}</div>
                <div class="text-xs text-text-dim truncate">{row.desc}</div>
              </div>
            </div>
            <button
              onclick={() => toggleFeature(row.key)}
              role="switch"
              aria-checked={settings.enabledFeatures[row.key]}
              class="relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 {settings.enabledFeatures[row.key]
                ? 'bg-accent'
                : 'bg-bg-hover border border-border'}"
            >
              <span
                class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transition-transform duration-200 {settings.enabledFeatures[row.key]
                  ? 'translate-x-4'
                  : 'translate-x-0'}"
              ></span>
            </button>
          </div>
        {/each}
      </div>
    </section>

    <!-- ===== L1-6: Profile 导入导出 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Download size={13} />
        Profile
      </h3>
      <div class="grid grid-cols-3 gap-2">
        <button
          onclick={exportProfileToFile}
          class="flex flex-col items-center gap-1 py-2.5 rounded-mac-sm border border-border bg-bg-hover/40 hover:bg-bg-hover text-text-dim hover:text-text transition-all"
        >
          <Download size={15} />
          <span class="text-xs">导出</span>
        </button>
        <button
          onclick={importProfileFromFile}
          class="flex flex-col items-center gap-1 py-2.5 rounded-mac-sm border border-border bg-bg-hover/40 hover:bg-bg-hover text-text-dim hover:text-text transition-all"
        >
          <Upload size={15} />
          <span class="text-xs">导入</span>
        </button>
        <button
          onclick={resetToDefault}
          class="flex flex-col items-center gap-1 py-2.5 rounded-mac-sm border border-border bg-bg-hover/40 hover:bg-bg-hover text-text-dim hover:text-error transition-all"
        >
          <RotateCcw size={15} />
          <span class="text-xs">重置</span>
        </button>
      </div>
    </section>

    <div class="h-2"></div>
  </div>
</div>

