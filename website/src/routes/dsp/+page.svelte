<script lang="ts">
  import { i18n } from '$lib/i18n/index.svelte';
</script>

<svelte:head>
  <title>Vesper DSP - {i18n.t.dspDetail.title}</title>
</svelte:head>

<section class="pt-32 pb-24 px-6 relative z-10 max-w-4xl mx-auto space-y-16">
  <!-- Header -->
  <div class="text-center space-y-4">
    <h1 class="text-5xl md:text-7xl font-semibold tracking-tighter text-[#f5f5f7]">Vesper DSP</h1>
    <p class="text-xl text-[#86868b] tracking-tight">{i18n.t.dspDetail.title}</p>
  </div>

  <!-- Intro -->
  <div class="text-lg md:text-xl text-[#86868b] font-medium leading-relaxed text-center max-w-3xl mx-auto tracking-tight">
    {i18n.t.dspDetail.intro}
  </div>

  <!-- Features Detail -->
  <div class="grid md:grid-cols-2 gap-6">
    <div class="p-10 rounded-[32px] bg-white/[0.02] border border-white/[0.08] shadow-[inset_0_1px_1px_rgba(255,255,255,0.1),0_8px_32px_rgba(0,0,0,0.4)] backdrop-blur-[60px]">
      <h3 class="text-2xl font-semibold tracking-tight text-[#f5f5f7] mb-4">{i18n.t.dspDetail.box1Title}</h3>
      <p class="text-[#86868b] leading-relaxed font-medium text-[17px]">{i18n.t.dspDetail.box1Desc}</p>
    </div>
    <div class="p-10 rounded-[32px] bg-white/[0.02] border border-white/[0.08] shadow-[inset_0_1px_1px_rgba(255,255,255,0.1),0_8px_32px_rgba(0,0,0,0.4)] backdrop-blur-[60px]">
      <h3 class="text-2xl font-semibold tracking-tight text-[#f5f5f7] mb-4">{i18n.t.dspDetail.box2Title}</h3>
      <p class="text-[#86868b] leading-relaxed font-medium text-[17px]">{i18n.t.dspDetail.box2Desc}</p>
    </div>
  </div>

  <!-- Download -->
  <div class="flex justify-center pt-10">
    <a href="https://github.com/minseokk77/vesper-dsp/releases/latest" target="_blank" class="px-10 py-4 rounded-full bg-[#f5f5f7] text-black font-semibold text-[15px] tracking-wide hover:scale-105 active:scale-95 transition-transform">
      {i18n.t.dspDetail.downloadBtn}
    </a>
  </div>
</section>
