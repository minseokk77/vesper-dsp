<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  export let value: string | number | null = null;
  export let options: { value: string | number | null, label: string }[] = [];
  export let placeholder = '선택해주세요';
  export let align = 'left'; // 'left' | 'right'
  export let customClass = '';
  export let textClass = 'text-white/90';
  export let bgClass = 'bg-[#1C1C1E] border border-white/10 hover:bg-[#2C2C2E]';
  
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
    class="w-full flex items-center justify-between gap-3 {bgClass} text-sm font-medium outline-none rounded-xl px-4 py-3 cursor-pointer transition-all shadow-sm"
    on:click={toggle}
  >
    <span class="truncate {value === null ? 'opacity-50' : ''} {textClass}">
      {options.find(o => o.value === value)?.label || placeholder}
    </span>
    <svg class="w-4 h-4 opacity-50 flex-shrink-0 transition-transform {textClass} {isOpen ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
  </button>

  {#if isOpen}
    <div class="absolute {dropUp ? 'bottom-full mb-2 origin-bottom' : 'top-full mt-2 origin-top'} {align === 'right' ? 'right-0' : 'left-0'} min-w-full w-full max-h-[250px] overflow-y-auto bg-[#242426] border border-white/10 rounded-xl shadow-[0_8px_30px_rgb(0,0,0,0.8)] z-50 flex flex-col p-1.5 animate-in fade-in zoom-in-95 duration-150">
      {#each options as opt}
        <button 
          class="flex items-center px-3 py-3 w-full text-sm text-white/90 hover:bg-[#3A3A3C] rounded-lg transition-colors {value === opt.value ? 'bg-apple-blue/20 text-apple-blue font-bold' : ''}"
          on:click={() => selectOption(opt.value)}
          title={opt.label}
        >
          <span class="truncate w-full text-left leading-normal">{opt.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
