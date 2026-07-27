<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { i18n } from '$lib/i18n/index.svelte';

	let { children } = $props();
	
	let scrolled = $state(false);

	onMount(() => {
		const handleScroll = () => {
			scrolled = window.scrollY > 20;
		};
		window.addEventListener('scroll', handleScroll);
		return () => window.removeEventListener('scroll', handleScroll);
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<!-- Navigation -->
<nav class="fixed top-0 left-0 right-0 z-50 transition-all duration-500 {scrolled ? 'bg-black/70 backdrop-blur-3xl border-b border-white/[0.08] py-4' : 'bg-transparent py-8'}">
  <div class="max-w-6xl mx-auto px-6 flex justify-between items-center">
    <a href="{base}/" class="text-xl font-semibold tracking-tight text-white hover:opacity-80 transition-opacity">
      Vesper<span class="text-blue-500">.</span>
    </a>
    <div class="flex items-center gap-8 text-xs font-medium text-[#86868b] tracking-wide">
      <a href="{base}/dsp" class="hover:text-white transition-colors">{i18n.t.nav.dsp}</a>
      <a href="{base}/woofer" class="hover:text-white transition-colors">{i18n.t.nav.woofer}</a>
      <a href="https://github.com/minseokk77/vesper" target="_blank" class="hover:text-white transition-colors">{i18n.t.nav.github}</a>
      
      <!-- Language Toggle -->
      <button 
        class="ml-2 text-[10px] uppercase tracking-widest text-white/50 hover:text-white transition-colors"
        onclick={() => i18n.toggle()}
      >
        {i18n.lang === 'ko' ? 'EN' : 'KO'}
      </button>
    </div>
  </div>
</nav>

<!-- Ambient Liquid Background -->
<div class="fixed inset-0 z-[-1] overflow-hidden bg-black pointer-events-none">
  <!-- Glowing Orbs (macOS / VisionOS style ambient color) -->
  <div class="absolute top-[-10%] left-[-10%] w-[50vw] h-[50vw] rounded-full bg-indigo-600/30 blur-[120px] mix-blend-screen animate-pulse" style="animation-duration: 8s;"></div>
  <div class="absolute bottom-[-10%] right-[-10%] w-[60vw] h-[60vw] rounded-full bg-fuchsia-600/20 blur-[140px] mix-blend-screen animate-pulse" style="animation-duration: 10s; animation-delay: 2s;"></div>
  <div class="absolute top-[40%] left-[30%] w-[40vw] h-[40vw] rounded-full bg-blue-500/20 blur-[130px] mix-blend-screen animate-pulse" style="animation-duration: 12s; animation-delay: 4s;"></div>
</div>

<main class="min-h-screen relative z-10">
	{@render children()}
</main>

<!-- Footer -->
<footer class="border-t border-white/[0.08] py-16 px-6 mt-20 relative z-10 bg-black/50 backdrop-blur-md">
  <div class="max-w-6xl mx-auto flex flex-col md:flex-row justify-between items-center gap-8">
    <div class="flex flex-col items-center md:items-start gap-1">
      <div class="text-lg font-semibold tracking-tight text-[#f5f5f7]">Vesper<span class="text-blue-500">.</span></div>
      <span class="text-[#86868b] text-xs">{i18n.t.footer.copyright}</span>
    </div>
    <div class="text-xs text-[#86868b] flex flex-wrap gap-x-8 gap-y-4 mt-4 md:mt-0 justify-center">
      <a href="https://github.com/minseokk77/vesper" target="_blank" class="hover:text-[#f5f5f7] transition-colors">{i18n.t.footer.repo}</a>
      <a href="https://github.com/minseokk77/vesper-dsp" target="_blank" class="hover:text-[#f5f5f7] transition-colors">{i18n.t.footer.dspRel}</a>
      <a href="https://github.com/minseokk77/vesper-woofer" target="_blank" class="hover:text-[#f5f5f7] transition-colors">{i18n.t.footer.wooRel}</a>
      <a href="{base}/license" class="hover:text-[#f5f5f7] transition-colors">{i18n.t.footer.license}</a>
    </div>
  </div>
</footer>
