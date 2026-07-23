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
    Image,
    Wand2,
    Gauge,
    Droplet,
    Layers,
    Zap,
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
    THEME_PRESETS,
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
    setBackgroundType,
    setBackgroundField,
    setPanelOpacity,
    setBlurStrength,
    setMotionField,
    applyThemePreset,
    type Theme,
    type FontSize,
    type Layout,
    type Shortcuts,
    type EnabledModes,
    type EnabledFeatures,
    type BackgroundType,
    type MotionSettings,
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

  // ===== 背景类型选项 =====
  const bgTypeOptions: { value: BackgroundType; label: string; icon: ComponentType }[] = [
    { value: 'solid', label: '纯色', icon: Droplet },
    { value: 'gradient', label: '渐变', icon: Layers },
    { value: 'image', label: '图片', icon: Image },
    { value: 'animated', label: '动态', icon: Wand2 },
  ];

  // ===== 动效项 =====
  const motionItems: { key: keyof MotionSettings; label: string; desc: string; icon: ComponentType }[] = [
    { key: 'enabled', label: '总开关', desc: '一键关闭所有动效', icon: Zap },
    { key: 'cardHover3D', label: '卡片悬浮 3D', desc: '面板悬浮时轻微上抬', icon: Layers },
    { key: 'buttonRipple', label: '按钮涟漪', desc: '点击时波纹扩散', icon: Droplet },
    { key: 'listStagger', label: '列表交错入场', desc: '归档列表逐项浮现', icon: List },
    { key: 'modeTransition', label: '模式切换过渡', desc: '切换模式时滑动', icon: Sparkles },
    { key: 'progressShimmer', label: '进度条流光', desc: '进度条流光动画', icon: Gauge },
    { key: 'backgroundFlow', label: '背景流动', desc: '动态渐变持续流动', icon: Wand2 },
  ];

  // ===== 图片选择 (data URL) =====
  async function pickBackgroundImage(): Promise<void> {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        setBackgroundField('imagePath', dataUrl);
      };
      reader.readAsDataURL(file);
    };
    input.click();
  }

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

    <!-- ===== L1+: 预设主题包 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Wand2 size={13} />
        主题包
      </h3>
      <div class="grid grid-cols-2 gap-2">
        {#each THEME_PRESETS as preset (preset.id)}
          <button
            onclick={() => applyThemePreset(preset)}
            class="relative flex flex-col gap-1.5 p-2.5 rounded-mac-sm border transition-all duration-150 hover:scale-[1.02] active:scale-95 {settings.accentColor.toLowerCase() === preset.accent.toLowerCase() && settings.background.type === preset.background.type
              ? 'border-accent bg-accent/10'
              : 'border-border bg-bg-hover/40 hover:bg-bg-hover'}"
          >
            <!-- 预览色块 -->
            <div
              class="w-full h-10 rounded-mac-sm overflow-hidden"
              style="background: {preset.background.type === 'solid'
                ? (preset.background.solidColor || '#1c1c1e')
                : preset.background.type === 'gradient'
                  ? `linear-gradient(${preset.background.gradientAngle}deg, ${preset.background.gradientFrom}, ${preset.background.gradientTo})`
                  : preset.background.type === 'animated'
                    ? `linear-gradient(120deg, ${preset.background.animatedColors[0]}, ${preset.background.animatedColors[1]}, ${preset.background.animatedColors[2]})`
                    : preset.accent};"
            >
              <div class="w-3 h-3 rounded-full m-1.5" style="background: {preset.accent};"></div>
            </div>
            <div class="text-xs font-medium text-text">{preset.name}</div>
            <div class="text-[10px] text-text-dim leading-tight">{preset.desc}</div>
          </button>
        {/each}
      </div>
    </section>

    <!-- ===== L1+: 沉浸式背景 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Image size={13} />
        背景
      </h3>
      <!-- 背景类型 -->
      <div class="grid grid-cols-4 gap-1.5 mb-3">
        {#each bgTypeOptions as opt (opt.value)}
          {@const Icon = opt.icon}
          <button
            onclick={() => setBackgroundType(opt.value)}
            class="flex flex-col items-center gap-1 py-2 rounded-mac-sm border text-[11px] font-medium transition-all {settings.background.type === opt.value
              ? 'border-accent bg-accent/10 text-accent'
              : 'border-border bg-bg-hover/40 text-text-dim hover:bg-bg-hover'}"
          >
            <Icon size={14} />
            {opt.label}
          </button>
        {/each}
      </div>

      <!-- 纯色: 颜色选择 -->
      {#if settings.background.type === 'solid'}
        <div class="flex items-center gap-2">
          <div class="relative w-8 h-8 rounded-mac-sm overflow-hidden border border-border/60">
            <input
              type="color"
              value={settings.background.solidColor || '#1c1c1e'}
              oninput={(e) => setBackgroundField('solidColor', e.currentTarget.value)}
              class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
            />
          </div>
          <span class="text-xs text-text-dim">留空使用主题默认背景</span>
          {#if settings.background.solidColor}
            <button
              onclick={() => setBackgroundField('solidColor', '')}
              class="ml-auto text-xs text-text-dim hover:text-error transition-colors"
            >
              清除
            </button>
          {/if}
        </div>
      {/if}

      <!-- 渐变: 起止色 + 角度 -->
      {#if settings.background.type === 'gradient'}
        <div class="space-y-2.5">
          <div class="flex items-center gap-2">
            <label class="text-xs text-text-dim w-10">起点</label>
            <div class="relative w-7 h-7 rounded-mac-sm overflow-hidden border border-border/60">
              <input
                type="color"
                value={settings.background.gradientFrom}
                oninput={(e) => setBackgroundField('gradientFrom', e.currentTarget.value)}
                class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
              />
            </div>
            <span class="text-xs font-mono text-text-dim">{settings.background.gradientFrom}</span>
          </div>
          <div class="flex items-center gap-2">
            <label class="text-xs text-text-dim w-10">终点</label>
            <div class="relative w-7 h-7 rounded-mac-sm overflow-hidden border border-border/60">
              <input
                type="color"
                value={settings.background.gradientTo}
                oninput={(e) => setBackgroundField('gradientTo', e.currentTarget.value)}
                class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
              />
            </div>
            <span class="text-xs font-mono text-text-dim">{settings.background.gradientTo}</span>
          </div>
          <div class="flex items-center gap-2">
            <label class="text-xs text-text-dim w-10">角度</label>
            <input
              type="range"
              min="0"
              max="360"
              value={settings.background.gradientAngle}
              oninput={(e) => setBackgroundField('gradientAngle', parseInt(e.currentTarget.value, 10))}
              class="flex-1"
            />
            <span class="text-xs font-mono text-text-dim w-10 text-right">{settings.background.gradientAngle}°</span>
          </div>
        </div>
      {/if}

      <!-- 图片: 选择 + 不透明度 + 模糊 -->
      {#if settings.background.type === 'image'}
        <div class="space-y-2.5">
          <button
            onclick={pickBackgroundImage}
            class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-mac-sm border border-dashed border-border text-text-dim hover:text-accent hover:border-accent transition-all text-xs"
          >
            <Image size={14} />
            {settings.background.imagePath ? '更换图片' : '选择图片'}
          </button>
          {#if settings.background.imagePath}
            <div class="flex items-center gap-2">
              <label class="text-xs text-text-dim w-10">透明</label>
              <input
                type="range"
                min="0.1"
                max="1"
                step="0.05"
                value={settings.background.imageOpacity}
                oninput={(e) => setBackgroundField('imageOpacity', parseFloat(e.currentTarget.value))}
                class="flex-1"
              />
              <span class="text-xs font-mono text-text-dim w-10 text-right">{Math.round(settings.background.imageOpacity * 100)}%</span>
            </div>
            <div class="flex items-center gap-2">
              <label class="text-xs text-text-dim w-10">模糊</label>
              <input
                type="range"
                min="0"
                max="30"
                value={settings.background.imageBlur}
                oninput={(e) => setBackgroundField('imageBlur', parseInt(e.currentTarget.value, 10))}
                class="flex-1"
              />
              <span class="text-xs font-mono text-text-dim w-10 text-right">{settings.background.imageBlur}px</span>
            </div>
          {/if}
        </div>
      {/if}

      <!-- 动态渐变: 三色 + 速度 -->
      {#if settings.background.type === 'animated'}
        <div class="space-y-2.5">
          <div class="flex items-center gap-2">
            <label class="text-xs text-text-dim w-10">色 1</label>
            <div class="relative w-7 h-7 rounded-mac-sm overflow-hidden border border-border/60">
              <input
                type="color"
                value={settings.background.animatedColors[0]}
                oninput={(e) => setBackgroundField('animatedColors', [e.currentTarget.value, settings.background.animatedColors[1], settings.background.animatedColors[2]])}
                class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
              />
            </div>
            <label class="text-xs text-text-dim w-10 ml-2">色 2</label>
            <div class="relative w-7 h-7 rounded-mac-sm overflow-hidden border border-border/60">
              <input
                type="color"
                value={settings.background.animatedColors[1]}
                oninput={(e) => setBackgroundField('animatedColors', [settings.background.animatedColors[0], e.currentTarget.value, settings.background.animatedColors[2]])}
                class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
              />
            </div>
            <label class="text-xs text-text-dim w-10 ml-2">色 3</label>
            <div class="relative w-7 h-7 rounded-mac-sm overflow-hidden border border-border/60">
              <input
                type="color"
                value={settings.background.animatedColors[2]}
                oninput={(e) => setBackgroundField('animatedColors', [settings.background.animatedColors[0], settings.background.animatedColors[1], e.currentTarget.value])}
                class="absolute inset-0 w-[200%] h-[200%] -translate-x-1/4 -translate-y-1/4 cursor-pointer border-0 p-0"
              />
            </div>
          </div>
          <div class="flex items-center gap-2">
            <label class="text-xs text-text-dim w-10">速度</label>
            <input
              type="range"
              min="8"
              max="60"
              value={settings.background.animatedSpeed}
              oninput={(e) => setBackgroundField('animatedSpeed', parseInt(e.currentTarget.value, 10))}
              class="flex-1"
            />
            <span class="text-xs font-mono text-text-dim w-10 text-right">{settings.background.animatedSpeed}s</span>
          </div>
        </div>
      {/if}

      <!-- 暗角 (渐变/图片/动态通用) -->
      {#if settings.background.type !== 'solid'}
        <div class="flex items-center justify-between mt-3 pt-2 border-t border-border/30">
          <span class="text-xs text-text">暗角</span>
          <button
            onclick={() => setBackgroundField('vignette', !settings.background.vignette)}
            role="switch"
            aria-checked={settings.background.vignette}
            class="relative w-9 h-5 rounded-full transition-colors duration-200 {settings.background.vignette
              ? 'bg-accent'
              : 'bg-bg-hover border border-border'}"
          >
            <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transition-transform duration-200 {settings.background.vignette ? 'translate-x-4' : 'translate-x-0'}"></span>
          </button>
        </div>
      {/if}
    </section>

    <!-- ===== L1+: 透明度 + 模糊 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Droplet size={13} />
        毛玻璃
      </h3>
      <div class="space-y-3">
        <div class="flex items-center gap-2">
          <label class="text-xs text-text-dim w-12">透明度</label>
          <input
            type="range"
            min="0.3"
            max="1"
            step="0.02"
            value={settings.panelOpacity}
            oninput={(e) => setPanelOpacity(parseFloat(e.currentTarget.value))}
            class="flex-1"
          />
          <span class="text-xs font-mono text-text-dim w-10 text-right">{Math.round(settings.panelOpacity * 100)}%</span>
        </div>
        <div class="flex items-center gap-2">
          <label class="text-xs text-text-dim w-12">模糊</label>
          <input
            type="range"
            min="0"
            max="40"
            value={settings.blurStrength}
            oninput={(e) => setBlurStrength(parseInt(e.currentTarget.value, 10))}
            class="flex-1"
          />
          <span class="text-xs font-mono text-text-dim w-10 text-right">{settings.blurStrength}px</span>
        </div>
      </div>
    </section>

    <!-- ===== L1+: 动效 ===== -->
    <section>
      <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider mb-3 flex items-center gap-1.5">
        <Zap size={13} />
        动效
      </h3>
      <div class="space-y-1.5">
        {#each motionItems as row (row.key)}
          {@const Icon = row.icon}
          <div class="flex items-center justify-between px-3 py-2 rounded-mac-sm bg-bg-hover/40 border border-border {row.key === 'enabled' ? 'border-accent/30' : ''}">
            <div class="flex items-center gap-2.5 min-w-0">
              <Icon size={15} class={settings.motion[row.key] ? 'text-accent' : 'text-text-dim'} />
              <div class="min-w-0">
                <div class="text-sm text-text">{row.label}</div>
                <div class="text-xs text-text-dim truncate">{row.desc}</div>
              </div>
            </div>
            <button
              onclick={() => setMotionField(row.key, !settings.motion[row.key])}
              role="switch"
              aria-checked={settings.motion[row.key]}
              class="relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 {settings.motion[row.key]
                ? 'bg-accent'
                : 'bg-bg-hover border border-border'}"
            >
              <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow-sm transition-transform duration-200 {settings.motion[row.key] ? 'translate-x-4' : 'translate-x-0'}"></span>
            </button>
          </div>
        {/each}
      </div>
    </section>

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

