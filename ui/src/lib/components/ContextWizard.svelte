<script lang="ts">
  // 上下文感知压缩向导 (痛点①)
  // 四步选择: 收件人 → 传输方式 → 目标系统 → 优先级 → 推荐结果
  import {
    X,
    ChevronRight,
    ChevronLeft,
    Check,
    LoaderCircle,
    Sparkles,
    User,
    Users,
    Building,
    Globe,
    Mail,
    Send,
    Cloud,
    Usb,
    HardDrive,
    Monitor,
    Apple,
    Server,
    Smartphone,
    HelpCircle,
    Gauge,
    Zap,
    ShieldCheck,
    Package,
  } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import type { CompressionIntent, FormatSuggestion } from '$lib/tauri';
  import {
    appState,
    closeContextWizard,
    setWizardIntent,
    fetchContextSuggestion,
    applyContextSuggestion,
    pushLog,
  } from '$lib/stores/app.svelte';

  interface OptionDef<T extends string> {
    value: T;
    label: string;
    desc: string;
    icon: ComponentType;
  }

  const recipients: OptionDef<CompressionIntent['recipient']>[] = [
    { value: 'self', label: '自己备份', desc: '个人存档与备份', icon: User },
    { value: 'colleague', label: '同事协作', desc: '团队内部共享', icon: Users },
    { value: 'external', label: '外部客户', desc: '对外交付客户', icon: Building },
    { value: 'public', label: '公开下载', desc: '面向公众发布', icon: Globe },
  ];

  const transports: OptionDef<CompressionIntent['transport']>[] = [
    { value: 'email', label: '邮件', desc: '附件发送 (≤25MB)', icon: Mail },
    { value: 'im', label: '即时通讯', desc: '微信/Slack 等', icon: Send },
    { value: 'cloud', label: '网盘', desc: '云存储分享', icon: Cloud },
    { value: 'usb', label: 'U 盘', desc: '本地介质拷贝', icon: Usb },
    { value: 'local', label: '本地', desc: '本机内移动', icon: HardDrive },
  ];

  const targetSystems: OptionDef<CompressionIntent['target_os']>[] = [
    { value: 'windows', label: 'Windows', desc: '微软系统', icon: Monitor },
    { value: 'macos', label: 'macOS', desc: '苹果系统', icon: Apple },
    { value: 'linux', label: 'Linux', desc: '开源系统', icon: Server },
    { value: 'mobile', label: '手机', desc: 'iOS / Android', icon: Smartphone },
    { value: 'unknown', label: '未知', desc: '不确定目标', icon: HelpCircle },
  ];

  const priorities: OptionDef<CompressionIntent['priority']>[] = [
    { value: 'size', label: '最小体积', desc: '极致压缩比', icon: Package },
    { value: 'speed', label: '最快速度', desc: '快速处理与解压', icon: Zap },
    { value: 'compatibility', label: '最大兼容', desc: '跨平台无障碍', icon: Gauge },
    { value: 'security', label: '最高安全', desc: '加密 + 高强度', icon: ShieldCheck },
  ];

  // 步骤索引 → intent 字段名
  const stepKeys = ['recipient', 'transport', 'target_os', 'priority'] as const;
  function stepKey(step: number): keyof CompressionIntent {
    return stepKeys[step];
  }

  let step = $state(0); // 0..3 选择步骤, 4 结果
  let loading = $state(false);
  let suggestion = $state<FormatSuggestion | null>(null);
  let error = $state<string | null>(null);

  const steps = [
    { title: '收件人', subtitle: '这份压缩包要发给谁?', options: recipients },
    { title: '传输方式', subtitle: '将通过什么途径传递?', options: transports },
    { title: '目标系统', subtitle: '收件方使用什么系统?', options: targetSystems },
    { title: '优先级', subtitle: '最看重什么?', options: priorities },
  ];

  let intent = $derived(appState.wizardIntent);

  function selectOption<K extends keyof CompressionIntent>(
    key: K,
    value: CompressionIntent[K],
  ): void {
    setWizardIntent({ ...intent, [key]: value });
  }

  function next(): void {
    if (step < 3) {
      step += 1;
    } else {
      void fetchSuggestion();
    }
  }

  function back(): void {
    if (step > 0) step -= 1;
  }

  async function fetchSuggestion(): Promise<void> {
    loading = true;
    error = null;
    suggestion = null;
    try {
      const result = await fetchContextSuggestion(intent);
      if (result) {
        suggestion = result;
        step = 4;
      } else {
        error = '获取推荐失败, 请稍后重试';
      }
    } catch (e) {
      error = String(e);
      pushLog(`智能推荐异常: ${String(e)}`, 'error');
    } finally {
      loading = false;
    }
  }

  function applyResult(): void {
    if (suggestion) applyContextSuggestion(suggestion);
  }

  function handleClose(): void {
    closeContextWizard();
  }

  // 重置向导状态每次打开
  $effect(() => {
    if (appState.showContextWizard) {
      step = 0;
      loading = false;
      error = null;
      suggestion = null;
    }
  });

  let currentStep = $derived(steps[step]);
  let canProceed = $derived(step < 4);
  let isLastStep = $derived(step === 3);
  let isResult = $derived(step === 4);

  function isSelected(stepIdx: number, value: string): boolean {
    const key = stepKey(stepIdx);
    return (intent as unknown as Record<string, string>)[key as string] === value;
  }
</script>

{#if appState.showContextWizard}
  <!-- 遮罩层 -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/40 backdrop-blur-sm animate-fade-in"
    role="dialog"
    aria-modal="true"
    aria-labelledby="wizard-title"
    tabindex="-1"
    onkeydown={(e) => {
      if (e.key === 'Escape') handleClose();
    }}
  >
    <div
      class="glass rounded-mac-lg shadow-2xl border border-border/60 w-full max-w-[680px] max-h-[90vh] flex flex-col animate-scale-in overflow-hidden"
    >
      <!-- 头部 -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border/50">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-mac-sm bg-accent/15 flex items-center justify-center">
            <Sparkles class="text-accent" size={18} />
          </div>
          <div>
            <h2 id="wizard-title" class="text-base font-semibold text-text">智能格式推荐</h2>
            <p class="text-xs text-text-dim">基于场景上下文自动选择最佳压缩参数</p>
          </div>
        </div>
        <button
          onclick={handleClose}
          class="p-1.5 rounded-mac-sm text-text-dim hover:text-text hover:bg-bg-hover transition-all"
          aria-label="关闭"
        >
          <X size={18} />
        </button>
      </div>

      <!-- 进度指示 -->
      {#if canProceed}
        <div class="flex items-center gap-1.5 px-6 py-3 border-b border-border/40 bg-bg-hover/30">
          {#each steps as s, i (s.title)}
            <div class="flex items-center gap-1.5 flex-1">
              <div
                class="flex items-center gap-1.5 transition-all {i === step
                  ? 'text-accent font-medium'
                  : i < step
                    ? 'text-success'
                    : 'text-text-dim'}"
              >
                <div
                  class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-semibold border transition-all {i === step
                    ? 'border-accent bg-accent/15'
                    : i < step
                      ? 'border-success bg-success/15'
                      : 'border-border'}"
                >
                  {#if i < step}
                    <Check size={12} />
                  {:else}
                    {i + 1}
                  {/if}
                </div>
                <span class="text-xs">{s.title}</span>
              </div>
              {#if i < steps.length - 1}
                <div class="flex-1 h-px bg-border/60"></div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- 内容区 -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if loading}
          <div class="flex flex-col items-center justify-center py-12 gap-3">
            <LoaderCircle class="text-accent animate-spin" size={32} />
            <p class="text-sm text-text-dim">正在分析上下文, 生成推荐...</p>
          </div>
        {:else if isResult && suggestion}
          <div class="flex flex-col gap-5 animate-slide-up">
            <div class="text-center">
              <div class="w-12 h-12 mx-auto rounded-mac bg-success/15 flex items-center justify-center mb-2">
                <Check class="text-success" size={24} />
              </div>
              <h3 class="text-base font-semibold text-text">推荐方案</h3>
              <p class="text-xs text-text-dim mt-1">基于您的场景自动匹配</p>
            </div>

            <div class="grid grid-cols-2 gap-3">
              <div class="glass rounded-mac-sm border border-border/50 px-4 py-3">
                <div class="text-xs text-text-dim">格式</div>
                <div class="text-lg font-mono font-semibold text-text mt-0.5">{suggestion.format}</div>
              </div>
              <div class="glass rounded-mac-sm border border-border/50 px-4 py-3">
                <div class="text-xs text-text-dim">压缩级别</div>
                <div class="text-lg font-mono font-semibold text-text mt-0.5">{suggestion.level}</div>
              </div>
              <div class="glass rounded-mac-sm border border-border/50 px-4 py-3">
                <div class="text-xs text-text-dim">分卷大小</div>
                <div class="text-lg font-mono font-semibold text-text mt-0.5">
                  {suggestion.split_size ?? '不分卷'}
                </div>
              </div>
              <div class="glass rounded-mac-sm border border-border/50 px-4 py-3">
                <div class="text-xs text-text-dim">UTF-8 文件名</div>
                <div class="text-lg font-mono font-semibold text-text mt-0.5">
                  {suggestion.use_utf8 ? '启用' : '关闭'}
                </div>
              </div>
            </div>

            <div class="glass rounded-mac-sm border border-border/50 px-4 py-3">
              <div class="text-xs text-text-dim mb-1">推荐理由</div>
              <p class="text-sm text-text leading-relaxed">{suggestion.reason}</p>
            </div>

            <div class="glass rounded-mac-sm border border-accent/30 bg-accent/5 px-4 py-3">
              <div class="text-xs text-text-dim mb-1.5">您选择的上下文</div>
              <div class="flex flex-wrap gap-1.5">
                <span class="text-xs px-2 py-0.5 rounded-full bg-bg-hover text-text-dim">
                  {recipients.find((r) => r.value === intent.recipient)?.label}
                </span>
                <span class="text-xs px-2 py-0.5 rounded-full bg-bg-hover text-text-dim">
                  {transports.find((t) => t.value === intent.transport)?.label}
                </span>
                <span class="text-xs px-2 py-0.5 rounded-full bg-bg-hover text-text-dim">
                  {targetSystems.find((t) => t.value === intent.target_os)?.label}
                </span>
                <span class="text-xs px-2 py-0.5 rounded-full bg-bg-hover text-text-dim">
                  {priorities.find((p) => p.value === intent.priority)?.label}
                </span>
              </div>
            </div>
          </div>
        {:else if canProceed}
          <div class="flex flex-col gap-4 animate-slide-up">
            <div>
              <h3 class="text-base font-semibold text-text">{currentStep.title}</h3>
              <p class="text-sm text-text-dim mt-0.5">{currentStep.subtitle}</p>
            </div>
            <div class="grid grid-cols-2 gap-2.5">
              {#each currentStep.options as opt (opt.value)}
                {@const Icon = opt.icon}
                {@const selected = isSelected(step, opt.value)}
                <button
                  onclick={() => selectOption(stepKey(step), opt.value)}
                  class="flex items-start gap-3 p-3.5 rounded-mac-sm border text-left transition-all duration-150 active:scale-[0.98] {selected
                    ? 'border-accent bg-accent/10 ring-1 ring-accent/30'
                    : 'border-border bg-bg-hover/40 hover:bg-bg-hover hover:border-border'}"
                >
                  <div
                    class="w-9 h-9 rounded-mac-sm flex items-center justify-center flex-shrink-0 transition-colors {selected
                      ? 'bg-accent text-white'
                      : 'bg-bg-hover text-text-dim'}"
                  >
                    <Icon size={18} />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-medium text-text">{opt.label}</div>
                    <div class="text-xs text-text-dim mt-0.5">{opt.desc}</div>
                  </div>
                  {#if selected}
                    <Check class="text-accent flex-shrink-0 mt-1" size={16} />
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {:else if error}
          <div class="flex flex-col items-center justify-center py-12 gap-3">
            <p class="text-sm text-error">{error}</p>
            <button
              onclick={() => void fetchSuggestion()}
              class="btn-secondary text-xs"
            >
              重试
            </button>
          </div>
        {/if}
      </div>

      <!-- 底部按钮 -->
      <div class="flex items-center justify-between px-6 py-4 border-t border-border/50 bg-bg-hover/30">
        <div class="text-xs text-text-dim">
          {#if isResult}
            推荐已就绪
          {:else if canProceed}
            步骤 {step + 1} / {steps.length}
          {/if}
        </div>
        <div class="flex items-center gap-2">
          {#if isResult}
            <button
              onclick={() => (step = 3)}
              class="btn-secondary text-sm"
            >
              <ChevronLeft class="inline-block -mt-0.5 mr-1" size={16} />
              返回修改
            </button>
            <button
              onclick={applyResult}
              class="btn-primary text-sm"
            >
              <Check class="inline-block -mt-0.5 mr-1" size={16} />
              应用推荐
            </button>
          {:else}
            {#if step > 0}
              <button
                onclick={back}
                class="btn-secondary text-sm"
                disabled={loading}
              >
                <ChevronLeft class="inline-block -mt-0.5 mr-1" size={16} />
                上一步
              </button>
            {/if}
            <button
              onclick={next}
              class="btn-primary text-sm"
              disabled={loading}
            >
              {#if isLastStep}
                <Sparkles class="inline-block -mt-0.5 mr-1" size={16} />
                生成推荐
              {:else}
                下一步
                <ChevronRight class="inline-block -mt-0.5 ml-1" size={16} />
              {/if}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
