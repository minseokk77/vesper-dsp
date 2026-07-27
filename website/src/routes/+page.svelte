<script lang="ts">
  import { onMount } from 'svelte';

  let scrolled = $state(false);

  onMount(() => {
    const handleScroll = () => {
      scrolled = window.scrollY > 20;
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  });
</script>

<svelte:head>
  <title>Vesper - The Ultimate Audio Ecosystem</title>
  <meta name="description" content="Vesper DSP & Woofer. High-fidelity audio processing and delay synchronization utility." />
</svelte:head>

<!-- Navigation -->
<nav class="fixed top-0 left-0 right-0 z-50 transition-all duration-300 {scrolled ? 'bg-[#0E0E10]/70 backdrop-blur-xl border-b border-white/10 py-3' : 'bg-transparent py-6'}">
  <div class="max-w-6xl mx-auto px-6 flex justify-between items-center">
    <div class="text-xl font-bold tracking-tighter text-white">
      Vesper<span class="text-blue-500">.</span>
    </div>
    <div class="flex items-center gap-6 text-sm font-medium text-white/70">
      <a href="#dsp" class="hover:text-white transition-colors">DSP</a>
      <a href="#woofer" class="hover:text-white transition-colors">Woofer</a>
      <a href="https://github.com/minseokk77/vesper" target="_blank" class="hover:text-white transition-colors">GitHub</a>
    </div>
  </div>
</nav>

<!-- Hero Section -->
<section class="relative min-h-[90vh] flex flex-col items-center justify-center pt-32 pb-20 px-6 overflow-hidden">
  <!-- Background Glows -->
  <div class="absolute top-1/3 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-blue-600/20 rounded-full blur-[120px] pointer-events-none"></div>
  <div class="absolute bottom-0 left-1/4 w-[400px] h-[400px] bg-purple-600/10 rounded-full blur-[100px] pointer-events-none"></div>

  <div class="z-10 flex flex-col items-center text-center max-w-4xl mx-auto space-y-6">
    <div class="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/5 border border-white/10 backdrop-blur-md mb-4">
      <span class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
      <span class="text-[11px] font-medium tracking-wide text-white/80 uppercase">Available for Windows</span>
    </div>
    
    <h1 class="text-6xl md:text-8xl font-bold tracking-tighter text-white leading-[1.1]">
      The Ultimate <br/> 
      <span class="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-500">Audio Ecosystem.</span>
    </h1>
    
    <p class="text-lg md:text-xl text-white/60 max-w-2xl font-light leading-relaxed mt-6">
      Experience studio-grade digital signal processing and flawless subwoofer delay synchronization, perfectly integrated for your desktop.
    </p>

    <div class="flex flex-col sm:flex-row items-center gap-4 pt-8">
      <a href="#dsp" class="px-8 py-4 rounded-2xl bg-white text-black font-semibold tracking-wide hover:scale-105 active:scale-95 transition-all shadow-[0_0_40px_rgba(255,255,255,0.2)]">
        Explore DSP
      </a>
      <a href="#woofer" class="px-8 py-4 rounded-2xl bg-white/5 border border-white/10 text-white font-semibold tracking-wide hover:bg-white/10 hover:scale-105 active:scale-95 transition-all backdrop-blur-md">
        Explore Woofer
      </a>
    </div>
  </div>
</section>

<!-- Features Grid -->
<section class="py-24 px-6 relative z-10">
  <div class="max-w-6xl mx-auto grid md:grid-cols-2 gap-8">
    
    <!-- DSP Card -->
    <div id="dsp" class="group relative rounded-3xl bg-gradient-to-b from-white/[0.08] to-white/[0.02] border border-white/10 p-10 overflow-hidden backdrop-blur-2xl hover:border-white/20 transition-colors">
      <div class="absolute inset-0 bg-gradient-to-br from-blue-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
      
      <div class="relative z-10 flex flex-col h-full">
        <div class="w-12 h-12 rounded-2xl bg-blue-500/20 flex items-center justify-center border border-blue-500/30 mb-8">
          <svg class="w-6 h-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"></path></svg>
        </div>
        
        <h2 class="text-3xl font-bold tracking-tight text-white mb-4">Vesper DSP</h2>
        <p class="text-white/60 leading-relaxed font-light mb-8 flex-1">
          System-wide digital signal processor and virtual audio cable. Integrates seamlessly with AutoEq and Spinorama to provide pixel-perfect acoustic correction.
        </p>
        
        <ul class="space-y-3 mb-10">
          <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-blue-400">✓</span> Virtual Audio Routing</li>
          <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-blue-400">✓</span> AutoEQ & Spinorama Profiles</li>
          <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-blue-400">✓</span> Real-time Resampling</li>
        </ul>
        
        <a href="https://github.com/minseokk77/vesper-dsp/releases/latest" target="_blank" class="inline-flex items-center justify-center w-full py-4 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold transition-colors">
          Download DSP for Windows
        </a>
      </div>
    </div>

    <!-- Woofer Card -->
    <div id="woofer" class="group relative rounded-3xl bg-gradient-to-b from-white/[0.08] to-white/[0.02] border border-white/10 p-10 overflow-hidden backdrop-blur-2xl hover:border-white/20 transition-colors">
      <div class="absolute inset-0 bg-gradient-to-bl from-purple-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
      
      <div class="relative z-10 flex flex-col h-full">
        <div class="w-12 h-12 rounded-2xl bg-purple-500/20 flex items-center justify-center border border-purple-500/30 mb-8">
          <svg class="w-6 h-6 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"></path></svg>
        </div>
        
        <h2 class="text-3xl font-bold tracking-tight text-white mb-4">Vesper Woofer</h2>
        <p class="text-white/60 leading-relaxed font-light mb-8 flex-1">
          The ultimate subwoofer and bass channel delay synchronization utility. Fix phase cancellation and achieve perfect low-end harmony in any room acoustic environment.
        </p>
        
        <ul class="space-y-3 mb-10">
          <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-purple-400">✓</span> Millisecond Precision Delay Sync</li>
          <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-purple-400">✓</span> Custom Crossover & Cut-off Filters</li>
          <li class="flex items-center gap-3 text-sm text-white/80"><span class="text-purple-400">✓</span> Seamless DSP Integration</li>
        </ul>
        
        <a href="https://github.com/minseokk77/vesper-woofer/releases/latest" target="_blank" class="inline-flex items-center justify-center w-full py-4 rounded-xl bg-purple-600 hover:bg-purple-500 text-white font-semibold transition-colors">
          Download Woofer for Windows
        </a>
      </div>
    </div>

  </div>
</section>

<!-- Footer -->
<footer class="border-t border-white/5 py-12 px-6 mt-12 relative z-10">
  <div class="max-w-6xl mx-auto flex flex-col md:flex-row justify-between items-center gap-6">
    <div class="flex items-center gap-2">
      <div class="text-lg font-bold tracking-tighter text-white">Vesper<span class="text-blue-500">.</span></div>
      <span class="text-white/30 text-sm">© 2026 minseokk77</span>
    </div>
    <div class="text-[11px] text-white/40 flex gap-6">
      <a href="https://github.com/minseokk77/vesper" target="_blank" class="hover:text-white transition-colors">Private Monorepo</a>
      <a href="https://github.com/minseokk77/vesper-dsp" target="_blank" class="hover:text-white transition-colors">DSP Releases</a>
      <a href="https://github.com/minseokk77/vesper-woofer" target="_blank" class="hover:text-white transition-colors">Woofer Releases</a>
    </div>
  </div>
</footer>

<style>
  :global(html) {
    scroll-behavior: smooth;
  }
</style>
