<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
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
<nav class="fixed top-0 left-0 right-0 z-50 transition-all duration-300 {scrolled ? 'bg-[#0E0E10]/70 backdrop-blur-xl border-b border-white/10 py-3' : 'bg-transparent py-6'}">
  <div class="max-w-6xl mx-auto px-6 flex justify-between items-center">
    <a href="/" class="text-xl font-bold tracking-tighter text-white hover:text-white/80 transition-colors">
      Vesper<span class="text-blue-500">.</span>
    </a>
    <div class="flex items-center gap-6 text-sm font-medium text-white/70">
      <a href="/dsp" class="hover:text-white transition-colors">{i18n.t.nav.dsp}</a>
      <a href="/woofer" class="hover:text-white transition-colors">{i18n.t.nav.woofer}</a>
      <a href="https://github.com/minseokk77/vesper" target="_blank" class="hover:text-white transition-colors">{i18n.t.nav.github}</a>
      
      <!-- Language Toggle -->
      <button 
        class="ml-4 px-3 py-1 rounded-full bg-white/10 border border-white/20 hover:bg-white/20 text-white text-xs font-bold transition-colors uppercase"
        onclick={() => i18n.toggle()}
      >
        {i18n.lang === 'ko' ? 'EN' : 'KO'}
      </button>
    </div>
  </div>
</nav>

<main class="min-h-screen">
	{@render children()}
</main>

<!-- Footer -->
<footer class="border-t border-white/5 py-12 px-6 mt-12 relative z-10 bg-black/20">
  <div class="max-w-6xl mx-auto flex flex-col md:flex-row justify-between items-center gap-6">
    <div class="flex items-center gap-2">
      <div class="text-lg font-bold tracking-tighter text-white">Vesper<span class="text-blue-500">.</span></div>
      <span class="text-white/30 text-sm">{i18n.t.footer.copyright}</span>
    </div>
    <div class="text-[11px] text-white/40 flex gap-6">
      <a href="https://github.com/minseokk77/vesper" target="_blank" class="hover:text-white transition-colors">{i18n.t.footer.repo}</a>
      <a href="https://github.com/minseokk77/vesper-dsp" target="_blank" class="hover:text-white transition-colors">{i18n.t.footer.dspRel}</a>
      <a href="https://github.com/minseokk77/vesper-woofer" target="_blank" class="hover:text-white transition-colors">{i18n.t.footer.wooRel}</a>
    </div>
  </div>
</footer>
