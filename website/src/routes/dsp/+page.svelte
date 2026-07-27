<script lang="ts">
  import { i18n } from '$lib/i18n/index.svelte';
</script>

<svelte:head>
  <title>Vesper DSP - {i18n.t.dspDetail.title}</title>
</svelte:head>

<section class="pt-32 pb-24 px-6 relative z-10 max-w-4xl mx-auto space-y-16">
  <!-- Header -->
  <div class="text-center space-y-6">
    <div class="w-20 h-20 mx-auto rounded-3xl bg-blue-500/20 flex items-center justify-center border border-blue-500/30">
      <svg class="w-10 h-10 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"></path></svg>
    </div>
    <h1 class="text-5xl font-bold tracking-tight text-white">Vesper DSP</h1>
    <p class="text-xl text-blue-400">{i18n.t.dspDetail.title}</p>
  </div>

  <!-- Intro -->
  <div class="text-lg text-white/70 font-light leading-relaxed text-center max-w-3xl mx-auto">
    {i18n.t.dspDetail.intro}
  </div>

  <!-- Features Detail -->
  <div class="grid md:grid-cols-2 gap-8">
    <div class="p-8 rounded-3xl bg-gradient-to-b from-white/[0.08] to-white/[0.02] border border-white/10 backdrop-blur-2xl">
      <h3 class="text-2xl font-bold text-white mb-4">{i18n.t.dspDetail.box1Title}</h3>
      <p class="text-white/60 leading-relaxed font-light">{i18n.t.dspDetail.box1Desc}</p>
    </div>
    <div class="p-8 rounded-3xl bg-gradient-to-b from-white/[0.08] to-white/[0.02] border border-white/10 backdrop-blur-2xl">
      <h3 class="text-2xl font-bold text-white mb-4">{i18n.t.dspDetail.box2Title}</h3>
      <p class="text-white/60 leading-relaxed font-light">{i18n.t.dspDetail.box2Desc}</p>
    </div>
  </div>

  <!-- Download -->
  <div class="flex justify-center pt-8">
    <a href="https://github.com/minseokk77/vesper-dsp/releases/latest" target="_blank" class="px-12 py-5 rounded-2xl bg-blue-600 hover:bg-blue-500 text-white font-semibold tracking-wide hover:scale-105 active:scale-95 transition-all shadow-[0_0_40px_rgba(59,130,246,0.3)]">
      {i18n.t.dspDetail.downloadBtn}
    </a>
  </div>
</section>
