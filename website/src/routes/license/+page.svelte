<script lang="ts">
  import { i18n } from '$lib/i18n/index.svelte';
</script>

<svelte:head>
  <title>Vesper - {i18n.t.license.title}</title>
</svelte:head>

<section class="pt-32 pb-24 px-6 relative z-10 max-w-4xl mx-auto space-y-12">
  <!-- Header -->
  <div class="space-y-4">
    <div class="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/[0.04] border border-white/[0.08] backdrop-blur-md mb-2">
      <svg class="w-4 h-4 text-[#86868b]" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"></path></svg>
      <span class="text-[10px] font-semibold tracking-widest text-[#86868b] uppercase">Proprietary / All Rights Reserved</span>
    </div>
    <h1 class="text-4xl md:text-5xl font-semibold tracking-tighter text-[#f5f5f7]">{i18n.t.license.title}</h1>
    <p class="text-lg text-[#86868b] tracking-tight">{i18n.t.license.subtitle}</p>
  </div>

  <!-- Content -->
  <div class="p-8 md:p-12 rounded-[32px] bg-white/[0.02] border border-white/[0.08] shadow-[inset_0_1px_1px_rgba(255,255,255,0.1),0_8px_32px_rgba(0,0,0,0.4)] backdrop-blur-[60px] space-y-10">
    {#each i18n.t.license.terms as term}
      <div class="space-y-3">
        <h3 class="text-xl font-semibold tracking-tight text-[#f5f5f7]">{term.title}</h3>
        <p class="text-[#86868b] leading-relaxed font-medium text-[16px]">{term.content}</p>
      </div>
    {/each}
  </div>
</section>
