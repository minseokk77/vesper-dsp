<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  export let value: string | number | null = null;
  export let options: { value: string | number | null, label: string }[] = [];
  export let placeholder = '선택해주세요';
  export let align = 'left'; // 'left' | 'right'
  export let customClass = '';
  export let textClass = 'text-white/90';
  export let bgClass = 'bg-white/5 border border-white/5 hover:bg-white/10';
  
  export  let isOpen = false;
  let dropUp = false;
  let buttonElement: HTMLButtonElement;
  const dispatch = createEventDispatcher();

  function selectOption(val: string | number | null) {
    value = val;
    isOpen = false;
    dispatch('change', { value });
  }

  function closeOnOutsideClick() {
    if (isOpen) isOpen = false;
  }

  function toggle() {
    isOpen = !isOpen;
    if (isOpen && buttonElement) {
      // Calculate available space below the button
      const rect = buttonElement.getBoundingClientRect();
      const spaceBelow = window.innerHeight - rect.bottom;
      // If space is less than ~280px, open upwards
      dropUp = spaceBelow < 280;
    }
  }
</script>

<svelte:window on:mousedown={closeOnOutsideClick} />

<div class="relative {customClass}" role="presentation" on:mousedown|stopPropagation>
  <button 
    bind:this={buttonElement}
    type="button"
    class="w-full flex items-center justify-between gap-2 {bgClass} text-sm outline-none rounded-xl px-4 py-2 cursor-pointer transition-all duration-200 active:scale-95 shadow-sm"
    on:click={toggle}
  >
    <span class="truncate {value === null ? 'opacity-50' : ''} {textClass}">
      {options.find(o => o.value === value)?.label || placeholder}
    </span>
    <svg class="w-4 h-4 opacity-50 flex-shrink-0 transition-transform duration-300 {textClass} {isOpen ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m19 9-7 7-7-7"></path></svg>
  </button>

  {#if isOpen}
    <div class="absolute {dropUp ? 'bottom-full mb-2 origin-bottom' : 'top-full mt-2 origin-top'} {align === 'right' ? 'right-0' : 'left-0'} min-w-max w-full max-h-[280px] overflow-y-auto glass-backdrop border border-[var(--color-widget-border)] rounded-xl shadow-2xl z-50 flex flex-col p-1.5 animate-in fade-in zoom-in-95 duration-200">
      {#each options as opt}
        <button 
          class="flex items-center justify-between px-3 py-2 w-full text-sm text-white/90 hover:bg-white/10 rounded-lg transition-colors group"
          on:click={() => selectOption(opt.value)}
          title={opt.label}
        >
          <span class="truncate pr-4 text-left leading-normal {value === opt.value ? 'font-medium text-white' : 'text-white/70 group-hover:text-white'}">{opt.label}</span>
          {#if value === opt.value}
            <svg class="w-4 h-4 text-white flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
