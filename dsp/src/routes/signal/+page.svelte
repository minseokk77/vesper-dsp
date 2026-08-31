<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { page } from '$app/stores';

  let selectedSource = 'Virtual Cable';
  let selectedOutput = 'Output Device';
  let isAsioMode = false;
  
  // Source Profile
  let sourceSampleRate = 48000;
  let sourceBitDepth = '24bit';
  let sourceChannels = 2;
  
  // Output Profile
  let headroomDb = -3.0;
  let sampleRate = 48000;
  let outputBitDepth = '32bit Float';
  let outputChannels = 2;

  // EQ Profile
  $: eqEnabled = $page.url.searchParams.get('eq') === 'true';
  $: eqProductName = $page.url.searchParams.get('eq_name') || '';
  
  $: isModal = $page.url.searchParams.get('is_modal') === 'true';
  
  function loadStates() {
    selectedSource = localStorage.getItem('vesper_dsp_source') || 'Virtual Audio Cable';
    selectedOutput = localStorage.getItem('vesper_dsp_output') || 'No Output Device';
    isAsioMode = localStorage.getItem('vesper_dsp_asio') === 'true';

    const h = localStorage.getItem('vesper_dsp_headroom'); 
    if (h) headroomDb = Number(h);

    // eqEnabled와 eqProductName은 부모 iframe 쿼리 파라미터에서 실시간으로 받음
    // 로컬 스토리지 읽는 부분 제거

    // 소스(입력 기기) 샘플레이트 및 비트 심도 조회
    if (selectedSource) {
      invoke<number>('get_device_sample_rate', { deviceName: selectedSource, isAsio: isAsioMode })
        .then((sr) => { sourceSampleRate = sr; })
        .catch(() => { sourceSampleRate = 48000; });
      invoke<string>('get_device_bit_depth', { deviceName: selectedSource, isAsio: isAsioMode })
        .then((bd) => { sourceBitDepth = bd; })
        .catch(() => { sourceBitDepth = '알 수 없음'; });
    }

    // 출력 샘플레이트 및 실제 비트 심도 백엔드 조회
    if (selectedOutput && selectedOutput !== 'No Output Device') {
      invoke<number>('get_device_sample_rate', { deviceName: selectedOutput, isAsio: isAsioMode })
        .then((sr) => { sampleRate = sr; })
        .catch(() => { sampleRate = 48000; });
      invoke<string>('get_device_bit_depth', { deviceName: selectedOutput, isAsio: isAsioMode })
        .then((bd) => { outputBitDepth = bd; })
        .catch((err) => { outputBitDepth = `오류 (${err})`; });
    } else {
      sampleRate = 48000;
      outputBitDepth = '-';
    }

    // DSP 엔진이 실행 중이면 실제 스트림 파라미터로 덮어쓰기 (메인 페이지가 localStorage에 저장)
    const streamInfoStr = localStorage.getItem('vesper_dsp_stream_info');
    if (streamInfoStr) {
      try {
        const info = JSON.parse(streamInfoStr);
        sourceSampleRate = info.source_sample_rate;
        sourceBitDepth = info.source_bit_depth;
        sourceChannels = info.source_channels;
        sampleRate = info.output_sample_rate;
        outputBitDepth = info.output_bit_depth;
        outputChannels = info.output_channels;
      } catch {}
    }
  }

  let unlistenUpdate: () => void;

  onMount(async () => {
    loadStates();
    
    window.addEventListener('storage', (e) => {
      loadStates();
    });

    // 실시간 상태 업데이트 구독
    unlistenUpdate = await listen('update-signal-path', () => {
      loadStates();
    });
  });

  onDestroy(() => {
    if (unlistenUpdate) unlistenUpdate();
  });

  // 장치명 예쁘게 포맷팅
  function formatDeviceName(name: string) {
    if (!name) return 'Unknown Device';
    let result = name;
    
    // 1. 괄호 안의 진짜 기기명 추출
    const match = name.match(/\(([^)]+)\)/);
    if (match) {
      result = match[1];
    }
    
    // 2. 윈도우 숫자 접두어 제거
    result = result.replace(/^\d+\s*-\s*/, '');
    
    return result.trim();
  }

  import { afterUpdate } from 'svelte';
  
  let containerRef: HTMLDivElement;

  afterUpdate(() => {
    if (isModal && typeof window !== 'undefined' && containerRef) {
      setTimeout(() => {
        // 정확한 내부 래퍼의 높이 측정
        const h = containerRef.scrollHeight;
        window.parent.postMessage({ type: 'resize_signal_modal', height: h }, '*');
      }, 50);
    }
  });
</script>

<div class="relative w-full h-screen bg-[#0E0E10] text-white overflow-hidden" style="box-shadow: inset 0 0 100px rgba(0,0,0,0.5);">
  <div bind:this={containerRef} class="liquid-glass w-full min-h-full flex flex-col relative">
    
    <!-- Header (Drag Region) -->
    {#if !isModal}
      <div 
        class="flex items-center justify-between p-5 pb-3 border-b border-white/10 z-10 sticky top-0 bg-[#0E0E10]/80 backdrop-blur-md cursor-grab active:cursor-grabbing"
        on:mousedown={() => getCurrentWindow().startDragging()}
      >
        <h1 class="text-lg font-bold tracking-tight text-white/90">Signal Path</h1>
        <button 
          on:click={() => getCurrentWindow().close()} 
          on:mousedown|stopPropagation
          class="w-5 h-5 rounded-full bg-white/10 hover:bg-red-500 transition-colors flex items-center justify-center"
        >
          <svg class="w-3 h-3 text-white/50 opacity-100" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>
    {/if}

    <!-- Signal Node Tree -->
    <div class="p-4 pb-4 flex flex-col relative overflow-hidden 
                [&>div:not(:last-child)]:after:content-[''] 
                [&>div:not(:last-child)]:after:absolute 
                [&>div:not(:last-child)]:after:left-[13px] 
                [&>div:not(:last-child)]:after:top-7 
                [&>div:not(:last-child)]:after:-bottom-2 
                [&>div:not(:last-child)]:after:w-[2px] 
                [&>div:not(:last-child)]:after:bg-white/10 
                [&>div:not(:last-child)]:after:z-0">

      <!-- Node 1: Source -->
      <div class="flex items-start gap-5 relative h-[64px] mb-2">
        <div class="w-7 h-7 rounded-full bg-black flex items-center justify-center border border-white/20 z-10 shadow-lg shadow-white/5 shrink-0 text-[8px] font-bold">SRC</div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">오디오 소스 <span class="text-apple-blue">✨</span></span>
          <span class="text-[11px] text-apple-blue font-medium mt-1">Windows System Audio (순정 윈도우 사운드)</span>
          <span class="text-[10px] text-white/40 mt-1">Direct In-Place Buffer (가상 케이블 불필요) · {sourceSampleRate / 1000}kHz {sourceChannels}ch</span>
        </div>
      </div>

      <!-- Node 2: Bit Depth Conversion -->
      <div class="flex items-start gap-5 relative h-[64px] mb-2">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">비트심도 변경 <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">{sourceBitDepth} ➡️ 64bit Float (DSP)</span>
        </div>
      </div>

      <!-- Node 3: Headroom (Global Path) -->
      <div class="flex items-start gap-5 relative h-[64px] mb-2">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">헤드룸 조정 <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">{headroomDb.toFixed(2)} dB (클리핑 방지)</span>
        </div>
      </div>

      <!-- Node 4: Sample Rate Conversion -->
      <div class="flex items-start gap-5 relative h-[64px] mb-2">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">리샘플링 (Sample Rate) <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">{sourceSampleRate / 1000}kHz ➡️ {sampleRate / 1000}kHz</span>
          <span class="text-[10px] text-white/40 mt-1">{isAsioMode ? 'ASIO / Exclusive' : 'WASAPI / Shared'}</span>
        </div>
      </div>

      <!-- Node 4.5: Output EQ (Optional) -->
      {#if eqEnabled}
        <div class="flex items-start gap-5 relative h-[64px] mb-2">
          <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
            <div class="w-1.5 h-1.5 rounded-full bg-green-400 shadow-[0_0_8px_rgba(74,222,128,0.8)]"></div>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90 flex items-center gap-2">
              헤드폰 EQ <span class="text-green-400">✨</span>
            </span>
            <span class="text-[11px] text-green-400 font-medium mt-1">64bit Float DSP Processing</span>
            <span class="text-[10px] text-white/40 mt-1">
              {eqProductName ? `Target: ${eqProductName}` : 'Custom 3-Band EQ'}
            </span>
          </div>
        </div>
      {/if}


      <!-- Node 5: Output Bit Depth Re-Conversion -->
      <div class="flex items-start gap-5 relative h-[64px] mb-2">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">비트심도 변경 <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">64bit Float (DSP) ➡️ {outputBitDepth === 'F32' ? '32bit Float (OS 믹서)' : outputBitDepth}</span>
        </div>
      </div>
      
      <!-- Output 1: Earphone/DAC -->
      <div class="flex items-start gap-5 relative mb-7">
        <div class="w-7 h-7 rounded-full bg-white flex items-center justify-center z-10 shadow-lg shadow-white/20 shrink-0 text-black">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.121-2.121a3 3 0 01-4.243 0m4.243 0a8 8 0 010 11.314M4 12h8m-8 0a2 2 0 100 4 2 2 0 000-4z"></path></svg>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90">Primary Output</span>
          <span class="text-[11px] text-apple-blue font-medium mt-1">{formatDeviceName(selectedOutput)}</span>
          <span class="text-[10px] text-white/40 mt-1">OS 믹서 / Core Audio</span>
        </div>
      </div>

    </div>
  </div>
</div>
