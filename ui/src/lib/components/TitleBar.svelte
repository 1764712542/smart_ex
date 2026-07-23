<script lang="ts">
  import { Settings, Sun, Moon, Workflow } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    title?: string;
    theme?: 'dark' | 'light';
    onToggleTheme?: () => void;
    onOpenSettings?: () => void;
    onOpenWorkflowEditor?: () => void;
  }

  let {
    title = 'SmartEx',
    theme = 'dark',
    onToggleTheme,
    onOpenSettings,
    onOpenWorkflowEditor,
  }: Props = $props();

  let ThemeIcon = $derived(
    (theme === 'dark' ? Sun : Moon) as ComponentType
  );
</script>

<div
  class="flex items-center justify-between h-11 pl-[70px] pr-3 border-b border-border/50 glass select-none flex-shrink-0"
  data-tauri-drag-region
>
  <!-- Left: 系统红绿灯由 macOS 渲染 (titleBarStyle: overlay), 留出 70px 空间 -->

  <!-- Center: Title -->
  <div class="text-sm font-semibold text-text-dim" data-tauri-drag-region>
    {title}
  </div>

  <!-- Right: Actions -->
  <div class="flex items-center gap-1 w-24 justify-end">
    <IconButton icon={Workflow} label="工作流编排器" onclick={onOpenWorkflowEditor} size={16} />
    <IconButton icon={ThemeIcon} label="Toggle theme" onclick={onToggleTheme} size={16} />
    <IconButton icon={Settings} label="Settings" onclick={onOpenSettings} size={16} />
  </div>
</div>
