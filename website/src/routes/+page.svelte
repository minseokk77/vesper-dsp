<script lang="ts">
  import { i18n } from '$lib/i18n/index.svelte';
</script>

<svelte:head>
  <title>Vesper - The Ultimate Audio Ecosystem</title>
  <meta name="description" content="Vesper DSP & Woofer. High-fidelity audio processing and delay synchronization utility." />
</svelte:head>

<!-- Hero Section -->
<section class="relative min-h-screen flex flex-col items-center justify-center pt-32 pb-20 px-6 overflow-hidden">
  <!-- Background Glows -->
  <div class="absolute top-1/3 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-blue-600/20 rounded-full blur-[120px] pointer-events-none"></div>
  <div class="absolute bottom-0 left-1/4 w-[400px] h-[400px] bg-purple-600/10 rounded-full blur-[100px] pointer-events-none"></div>

  <div class="z-10 flex flex-col items-center text-center max-w-4xl mx-auto space-y-6">
    <div class="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/5 border border-white/10 backdrop-blur-md mb-4">
      <span class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
      <span class="text-[11px] font-medium tracking-wide text-white/80 uppercase">{i18n.t.hero.badge}</span>
    </div>
    
    <h1 class="text-6xl md:text-8xl font-bold tracking-tighter text-white leading-[1.1]">
      {i18n.t.hero.title1} <br/> 
      <span class="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-500">{i18n.t.hero.title2}</span>
    </h1>
    
    <p class="text-lg md:text-xl text-white/60 max-w-2xl font-light leading-relaxed mt-6">
      {i18n.t.hero.subtitle}
    </p>

    <div class="flex flex-col sm:flex-row items-center gap-4 pt-8">
      <a href="/dsp" class="px-8 py-4 rounded-2xl bg-white text-black font-semibold tracking-wide hover:scale-105 active:scale-95 transition-all shadow-[0_0_40px_rgba(255,255,255,0.2)]">
        {i18n.t.hero.btnDsp}
      </a>
      <a href="/woofer" class="px-8 py-4 rounded-2xl bg-white/5 border border-white/10 text-white font-semibold tracking-wide hover:bg-white/10 hover:scale-105 active:scale-95 transition-all backdrop-blur-md">
        {i18n.t.hero.btnWoofer}
      </a>
    </div>
  </div>
</section>

<!-- Features Grid -->
<section class="pb-24 px-6 relative z-10 -mt-20">
  <div class="max-w-6xl mx-auto grid md:grid-cols-2 gap-8">
    
    <!-- DSP Card -->
    <a href="/dsp" class="group relative rounded-3xl bg-gradient-to-b from-white/[0.08] to-white/[0.02] border border-white/10 p-10 overflow-hidden backdrop-blur-2xl hover:border-white/20 transition-all hover:-translate-y-2 block">
      <div class="absolute inset-0 bg-gradient-to-br from-blue-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
      
      <div class="relative z-10 flex flex-col h-full">
        <div class="w-12 h-12 rounded-2xl bg-blue-500/20 flex items-center justify-center border border-blue-500/30 mb-8">
          <svg class="w-6 h-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"></path></svg>
        </div>
        
        <h2 class="text-3xl font-bold tracking-tight text-white mb-4">{i18n.t.home.dspTitle}</h2>
        <p class="text-white/60 leading-relaxed font-light mb-8 flex-1">
          {i18n.t.home.dspDesc}
        </p>
        
        <ul class="space-y-3">
          {#each i18n.t.home.dspFeatures as feature}
            <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-blue-400">✓</span> {feature}</li>
          {/each}
        </ul>
      </div>
    </a>

    <!-- Woofer Card -->
    <a href="/woofer" class="group relative rounded-3xl bg-gradient-to-b from-white/[0.08] to-white/[0.02] border border-white/10 p-10 overflow-hidden backdrop-blur-2xl hover:border-white/20 transition-all hover:-translate-y-2 block">
      <div class="absolute inset-0 bg-gradient-to-bl from-purple-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
      
      <div class="relative z-10 flex flex-col h-full">
        <div class="w-12 h-12 rounded-2xl bg-purple-500/20 flex items-center justify-center border border-purple-500/30 mb-8">
          <svg class="w-6 h-6 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"></path></svg>
        </div>
        
        <h2 class="text-3xl font-bold tracking-tight text-white mb-4">{i18n.t.home.wooferTitle}</h2>
        <p class="text-white/60 leading-relaxed font-light mb-8 flex-1">
          {i18n.t.home.wooferDesc}
        </p>
        
        <ul class="space-y-3">
          {#each i18n.t.home.wooferFeatures as feature}
            <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-purple-400">✓</span> {feature}</li>
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
