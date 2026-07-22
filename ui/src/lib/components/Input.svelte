<script lang="ts">
  interface Props {
    label?: string;
    hint?: string;
    error?: string;
    placeholder?: string;
    type?: string;
    value?: string;
    disabled?: boolean;
    id?: string;
    onchange?: (e: Event) => void;
    oninput?: (e: Event) => void;
  }

  let {
    label,
    hint,
    error,
    placeholder,
    type = 'text',
    value = $bindable(''),
    disabled = false,
    id = `input-${Math.random().toString(36).slice(2, 9)}`,
    onchange,
    oninput,
  }: Props = $props();
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <label for={id} class="text-sm font-medium text-text-dim">{label}</label>
  {/if}
  <input
    {id}
    {type}
    {placeholder}
    bind:value
    {disabled}
    {onchange}
    {oninput}
    class="px-3 py-2 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all {error
      ? 'border-error'
      : ''} {disabled ? 'opacity-50 cursor-not-allowed' : ''}"
  />
  {#if error}
    <span class="text-xs text-error">{error}</span>
  {:else if hint}
    <span class="text-xs text-text-dim">{hint}</span>
  {/if}
</div>
