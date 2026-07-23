<script lang="ts">
  // 沉浸式背景层 — 根据设置渲染纯色/渐变/图片/动态渐变
  // 所有视觉参数由 CSS 变量驱动 (applyBackground 设置)
  import { settings } from '$lib/stores/settings.svelte';

  let bgType = $derived(settings.background.type);
  let vignette = $derived(settings.background.vignette);
</script>

<div
  class="bg-layer {bgType === 'solid' ? 'bg-layer-solid' : bgType === 'gradient' ? 'bg-layer-gradient' : bgType === 'image' ? 'bg-layer-image' : 'bg-layer-animated'} {vignette && (bgType === 'gradient' || bgType === 'image' || bgType === 'animated') ? 'bg-layer-vignette' : ''}"
  aria-hidden="true"
></div>
