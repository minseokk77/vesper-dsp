<script lang="ts">
  import { base } from '$app/paths';
  import { i18n } from '$lib/i18n/index.svelte';
</script>

<svelte:head>
  <title>Vesper - The Ultimate Audio Ecosystem</title>
  <meta name="description" content="Vesper DSP & Woofer. High-fidelity audio processing and delay synchronization utility." />
</svelte:head>

<!-- Hero Section -->
<section class="relative min-h-[90vh] flex flex-col items-center justify-center pt-32 pb-20 px-6">
  <div class="z-10 flex flex-col items-center text-center max-w-4xl mx-auto space-y-8">
    <div class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-white/[0.04] border border-white/[0.08] backdrop-blur-md mb-2">
      <span class="text-[10px] font-semibold tracking-widest text-[#86868b] uppercase">{i18n.t.hero.badge}</span>
    </div>
    
    <h1 class="text-6xl md:text-8xl font-semibold tracking-tighter text-[#f5f5f7] leading-[1.05]">
      {i18n.t.hero.title1} <br/> 
      <span class="text-[#f5f5f7]">{i18n.t.hero.title2}</span>
    </h1>
    
    <p class="text-xl md:text-2xl text-[#86868b] max-w-2xl font-medium leading-snug mt-6 tracking-tight">
      {i18n.t.hero.subtitle}
    </p>

    <div class="flex flex-col sm:flex-row items-center gap-6 pt-10">
      <a href="{base}/dsp" class="px-8 py-3.5 rounded-full bg-[#f5f5f7] text-black font-semibold text-[15px] hover:scale-105 active:scale-95 transition-transform">
        {i18n.t.hero.btnDsp}
      </a>
      <a href="{base}/woofer" class="px-8 py-3.5 rounded-full bg-white/[0.08] text-white font-semibold text-[15px] hover:bg-white/[0.12] hover:scale-105 active:scale-95 transition-all">
        {i18n.t.hero.btnWoofer}
      </a>
    </div>
  </div>
</section>

<!-- Features Grid -->
<section class="pb-32 px-6 relative z-10 -mt-10">
  <div class="max-w-5xl mx-auto grid md:grid-cols-2 gap-6">
    
    <!-- DSP Card -->
    <a href="{base}/dsp" class="group relative rounded-[32px] bg-white/[0.02] border border-white/[0.08] shadow-[inset_0_1px_1px_rgba(255,255,255,0.1),0_8px_32px_rgba(0,0,0,0.4)] backdrop-blur-[60px] p-10 overflow-hidden hover:scale-[1.02] transition-all duration-500 block hover:bg-white/[0.04]">
      <div class="absolute inset-0 bg-gradient-to-br from-white/[0.08] to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
      <div class="relative z-10 flex flex-col h-full">
        <h2 class="text-3xl font-semibold tracking-tight text-[#f5f5f7] mb-3">{i18n.t.home.dspTitle}</h2>
        <p class="text-[#86868b] leading-relaxed font-medium mb-10 text-[17px] tracking-tight group-hover:text-white/80 transition-colors">
          {i18n.t.home.dspDesc}
        </p>
        
        <ul class="space-y-4 mt-auto">
          {#each i18n.t.home.dspFeatures as feature}
            <li class="flex items-center gap-3 text-[15px] font-medium text-[#f5f5f7]/90"><span class="text-[#86868b] group-hover:text-white transition-colors">→</span> {feature}</li>
          {/each}
        </ul>
      </div>
    </a>

    <!-- Woofer Card -->
    <a href="{base}/woofer" class="group relative rounded-[32px] bg-white/[0.02] border border-white/[0.08] shadow-[inset_0_1px_1px_rgba(255,255,255,0.1),0_8px_32px_rgba(0,0,0,0.4)] backdrop-blur-[60px] p-10 overflow-hidden hover:scale-[1.02] transition-all duration-500 block hover:bg-white/[0.04]">
      <div class="absolute inset-0 bg-gradient-to-br from-white/[0.08] to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
      <div class="relative z-10 flex flex-col h-full">
        <h2 class="text-3xl font-semibold tracking-tight text-[#f5f5f7] mb-3">{i18n.t.home.wooferTitle}</h2>
        <p class="text-[#86868b] leading-relaxed font-medium mb-10 text-[17px] tracking-tight group-hover:text-white/80 transition-colors">
          {i18n.t.home.wooferDesc}
        </p>
        
        <ul class="space-y-4 mt-auto">
          {#each i18n.t.home.wooferFeatures as feature}
            <li class="flex items-center gap-3 text-[15px] font-medium text-[#f5f5f7]/90"><span class="text-[#86868b] group-hover:text-white transition-colors">→</span> {feature}</li>
          {/each}
        </ul>
      </div>
    </a>

  </div>
</section>

<style>
  :global(html) {
    scroll-behavior: smooth;
  }
</style>
