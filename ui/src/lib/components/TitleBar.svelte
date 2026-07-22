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
  class="flex items-center justify-between h-11 px-3 border-b border-border/50 glass select-none flex-shrink-0"
  data-tauri-drag-region
>
  <!-- Left: Traffic lights placeholder -->
  <div class="flex items-center gap-2 w-24" data-tauri-drag-region>
    <div class="w-3 h-3 rounded-full bg-[#ff5f57] hover:brightness-110 transition-all cursor-pointer"></div>
    <div class="w-3 h-3 rounded-full bg-[#febc2e] hover:brightness-110 transition-all cursor-pointer"></div>
    <div class="w-3 h-3 rounded-full bg-[#28c840] hover:brightness-110 transition-all cursor-pointer"></div>
  </div>

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
