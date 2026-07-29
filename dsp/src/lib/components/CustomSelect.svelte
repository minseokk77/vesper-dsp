<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';

  type SelectValue = string | number | null;
  type SelectOption = { value: SelectValue; label: string };
  type Props = {
    value?: SelectValue;
    options?: SelectOption[];
    placeholder?: string;
    align?: 'left' | 'right';
    customClass?: string;
    textClass?: string;
    bgClass?: string;
    isOpen?: boolean;
  };

  let {
    value = $bindable(null),
    options = [],
    placeholder = '선택해주세요',
    align = 'left',
    customClass = '',
    textClass = 'text-white/90',
    bgClass = 'bg-white/5 border border-white/5 hover:bg-white/10',
    isOpen = $bindable(false)
  }: Props = $props();

  let buttonElement = $state<HTMLButtonElement>();
  let menuElement = $state<HTMLDivElement>();
  let menuStyle = $state('');
  let activeIndex = $state(-1);
  const dispatch = createEventDispatcher<{ change: { value: SelectValue } }>();

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      }
    };
  }

  function positionMenu() {
    if (!buttonElement) return;
    const rect = buttonElement.getBoundingClientRect();
    const gap = 8;
    const maxHeight = 280;
    const openUp = window.innerHeight - rect.bottom < maxHeight + gap && rect.top > maxHeight;
    const left = align === 'right' ? rect.right - rect.width : rect.left;
    const top = openUp ? Math.max(gap, rect.top - maxHeight - gap) : rect.bottom + gap;
    menuStyle = `position:fixed;left:${left}px;top:${top}px;width:${rect.width}px;max-height:${maxHeight}px;z-index:9999;`;
  }

  async function openMenu() {
    isOpen = true;
    activeIndex = Math.max(0, options.findIndex((option) => option.value === value));
    await tick();
    positionMenu();
  }

  function closeMenu() {
    isOpen = false;
    activeIndex = -1;
  }

  function toggle() {
    if (isOpen) closeMenu();
    else void openMenu();
  }

  function selectOption(option: SelectOption) {
    value = option.value;
    closeMenu();
    dispatch('change', { value });
    buttonElement?.focus();
  }

  function handleOutsidePointer(event: MouseEvent) {
    const target = event.target as Node;
    if (isOpen && !buttonElement?.contains(target) && !menuElement?.contains(target)) closeMenu();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeMenu();
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (!isOpen) void openMenu();
      else if (options[activeIndex]) selectOption(options[activeIndex]);
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!isOpen) {
        void openMenu();
        return;
      }
      const direction = event.key === 'ArrowDown' ? 1 : -1;
      activeIndex = (activeIndex + direction + options.length) % options.length;
    }
  }
</script>

<svelte:window onmousedown={handleOutsidePointer} onresize={positionMenu} onscroll={positionMenu} />

<div class="relative {customClass}" role="presentation">
  <button
    bind:this={buttonElement}
    type="button"
    class="w-full flex items-center justify-between gap-2 {bgClass} text-sm outline-none rounded-xl px-4 py-2 cursor-pointer transition-all duration-200 active:scale-95 shadow-sm"
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    onclick={toggle}
    onkeydown={handleKeydown}
  >
    <span class="truncate {value === null ? 'opacity-50' : ''} {textClass}">
      {options.find((option) => option.value === value)?.label || placeholder}
    </span>
    <svg class="w-4 h-4 opacity-50 flex-shrink-0 transition-transform duration-300 {textClass} {isOpen ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m19 9-7 7-7-7"></path></svg>
  </button>
</div>

{#if isOpen}
  <div
    bind:this={menuElement}
    use:portal
    style={menuStyle}
    role="listbox"
    class="overflow-y-auto glass-backdrop border border-[var(--color-widget-border)] rounded-xl shadow-2xl flex flex-col p-1.5 animate-in fade-in zoom-in-95 duration-200"
  >
    {#each options as option, index}
      <button
        type="button"
        role="option"
        aria-selected={value === option.value}
        class="flex items-center justify-between px-3 py-2 w-full text-sm hover:bg-white/10 rounded-lg transition-colors group {index === activeIndex ? 'bg-white/10' : ''}"
        onmouseenter={() => activeIndex = index}
        onclick={() => selectOption(option)}
        title={option.label}
      >
        <span class="truncate pr-4 text-left leading-normal {value === option.value ? 'font-medium text-white' : 'text-white/70 group-hover:text-white'}">{option.label}</span>
        {#if value === option.value}
          <svg class="w-4 h-4 text-white flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>
        {/if}
      </button>
    {/each}
  </div>
{/if}
