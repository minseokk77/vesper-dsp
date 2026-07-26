<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { page } from '$app/stores';

  let selectedSource = 'Virtual Cable';
  let target = 'earphone';
  let isModal = false;

  $: {
    if (typeof window !== 'undefined') {
      target = $page.url.searchParams.get('target') || 'earphone';
      isModal = $page.url.searchParams.get('is_modal') === 'true';
    }
  }
  let selectedEarphone = 'Earphones';
  let selectedSpeaker = 'Sub Woofer';
  
  // Source Profile
  let sourceSampleRate = 48000;
  let sourceBitDepth = '24bit';
  
  // Earphone Profile
  let headroomDb = -3.0;
  let sampleRate = 48000;
  let earphoneBitDepth = '32bit Float';
  
  // Speaker Profile
  let lpfHz = 80;
  let delayMs = 0;
  let speakerBitDepth = '32bit Float';

  function loadStates() {
    selectedSource = localStorage.getItem('ws_source') || 'Virtual Audio Cable';
    selectedEarphone = localStorage.getItem('ws_current_earphone') || 'No Earphone';
    selectedSpeaker = localStorage.getItem('ws_current_speaker') || 'No Speaker';

    const h = localStorage.getItem(`ws_headroom_${selectedEarphone}`); 
    if (h) headroomDb = Number(h);
    const l = localStorage.getItem(`ws_lpfHz_${selectedSpeaker}`); 
    if (l) lpfHz = Number(l);
    const d = localStorage.getItem(`ws_delayMs_${selectedSpeaker}`); 
    if (d) delayMs = Number(d);

    // 소스(입력 기기) 샘플레이트 및 비트 심도 조회
    if (selectedSource) {
      invoke<number>('get_device_sample_rate', { deviceName: selectedSource })
        .then((sr) => { sourceSampleRate = sr; })
        .catch(() => { sourceSampleRate = 48000; });
      invoke<string>('get_device_bit_depth', { deviceName: selectedSource })
        .then((bd) => { sourceBitDepth = bd; })
        .catch(() => { sourceBitDepth = '알 수 없음'; });
    }

    // 출력 샘플레이트 및 실제 비트 심도 백엔드 조회
    const fiioName = selectedEarphone.includes('FiiO') ? selectedEarphone : 
                     (selectedSpeaker.includes('FiiO') ? selectedSpeaker : '');
    
    if (fiioName) {
      invoke<number>('get_device_sample_rate', { deviceName: fiioName })
        .then((sr) => { sampleRate = sr; })
        .catch(() => { sampleRate = 48000; });
    } else {
      sampleRate = 48000;
    }

    if (selectedEarphone && selectedEarphone !== 'No Earphone') {
      invoke<string>('get_device_bit_depth', { deviceName: selectedEarphone })
        .then((bd) => { earphoneBitDepth = bd; })
        .catch((err) => { earphoneBitDepth = `오류 (${err})`; });
    } else {
      earphoneBitDepth = '-';
    }

    if (selectedSpeaker && selectedSpeaker !== 'No Speaker') {
      invoke<string>('get_device_bit_depth', { deviceName: selectedSpeaker })
        .then((bd) => { speakerBitDepth = bd; })
        .catch((err) => { speakerBitDepth = `오류 (${err})`; });
    } else {
      speakerBitDepth = '-';
    }
  }

  let unlistenTarget: () => void;
  let unlistenUpdate: () => void;

  onMount(async () => {
    loadStates();
    
    window.addEventListener('storage', (e) => {
      // 로컬스토리지가 바뀔때마다 UI 갱신 (Fallback)
      loadStates();
    });

    // 메인 창에서 넘어오는 타겟 전환 이벤트 구독
    unlistenTarget = await listen<{target: string}>('change-signal-target', (event) => {
      target = event.payload.target;
    });

    // 실시간 상태 업데이트 구독
    unlistenUpdate = await listen('update-signal-path', () => {
      loadStates();
    });
  });

  onDestroy(() => {
    if (unlistenTarget) unlistenTarget();
    if (unlistenUpdate) unlistenUpdate();
  });

  // 장치명 예쁘게 포맷팅 (예: "2- 스피커 (FiiO K11)" -> "FiiO K11")
  function formatDeviceName(name: string) {
    if (!name) return 'Unknown Device';
    let result = name;
    
    // 1. 괄호 안의 진짜 기기명 추출
    const match = name.match(/\(([^)]+)\)/);
    if (match) {
      result = match[1];
    }
    
    // 2. 윈도우 고질병인 '2- ', '3-' 같은 숫자 접두어 제거
    result = result.replace(/^\d+\s*-\s*/, '');
    
    return result.trim();
  }
</script>

<div class="relative w-full min-h-screen bg-[#0E0E10] text-white overflow-y-auto" style="box-shadow: inset 0 0 100px rgba(0,0,0,0.5);">
  <div class="liquid-glass w-full min-h-full flex flex-col relative">
    
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
    <div class="p-6 pb-12 flex flex-col relative overflow-hidden 
                [&>div:not(:last-child)]:after:content-[''] 
                [&>div:not(:last-child)]:after:absolute 
                [&>div:not(:last-child)]:after:left-[13px] 
                [&>div:not(:last-child)]:after:top-0 
                [&>div:not(:last-child)]:after:-bottom-7 
                [&>div:not(:last-child)]:after:w-[2px] 
                [&>div:not(:last-child)]:after:bg-white/5 
                [&>div:not(:last-child)]:after:z-0">

      <!-- Node 1: Source -->
      <div class="flex items-start gap-5 relative mb-7">
        <div class="w-7 h-7 rounded-full bg-black flex items-center justify-center border border-white/20 z-10 shadow-lg shadow-white/5 shrink-0 text-[8px] font-bold">SRC</div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">오디오 소스 <span class="text-apple-blue">✨</span></span>
          <span class="text-[11px] text-apple-blue font-medium mt-1">{formatDeviceName(selectedSource)}</span>
          <span class="text-[10px] text-white/40 mt-1">입력: {sourceSampleRate / 1000}kHz {sourceBitDepth === 'F32' ? '32bit Float (OS 믹서)' : sourceBitDepth} 2ch</span>
        </div>
      </div>

      <!-- Node 2: Bit Depth Conversion -->
      <div class="flex items-start gap-5 relative mb-7">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">비트심도 변경 <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">{sourceBitDepth} ➡️ 64bit Float (내부 고정밀 DSP)</span>
        </div>
      </div>

      <!-- Node 2.2: Headroom (Global Path) -->
      <div class="flex items-start gap-5 relative mb-7">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">헤드룸 조정 <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">{headroomDb.toFixed(2)} dB (클리핑 방지)</span>
        </div>
      </div>

      <!-- Node 2.5: Sample Rate Conversion -->
      <div class="flex items-start gap-5 relative mb-7">
        <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
        </div>
        <div class="flex flex-col pt-1">
          <span class="text-sm font-bold text-white/90 flex items-center gap-2">리샘플링 (Sample Rate) <span class="text-purple-400">✨</span></span>
          <span class="text-[11px] text-purple-400 font-medium mt-1">{sourceSampleRate / 1000}kHz ➡️ {sampleRate / 1000}kHz</span>
          <span class="text-[10px] text-white/40 mt-1">WASAPI / Shared</span>
        </div>
      </div>

      <!-- Splitter 영역 제거됨 -->

      {#if target === 'earphone'}
        <!-- Node 4: Phase Delay (Earphone Path) -->
        <div class="flex items-start gap-5 relative mb-7">
          <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
            <div class="w-1.5 h-1.5 rounded-full bg-apple-blue shadow-[0_0_8px_rgba(6,182,212,0.8)]"></div>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90 flex items-center gap-2">위상 지연 (Phase Delay) <span class="text-apple-blue">✨</span></span>
            <span class="text-[11px] text-apple-blue font-medium mt-1">{delayMs} ms (Primary Earphones)</span>
          </div>
        </div>
        
        <!-- Earphone Bit Depth Re-Conversion -->
        <div class="flex items-start gap-5 relative mb-7">
          <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
            <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90 flex items-center gap-2">비트심도 변경 <span class="text-purple-400">✨</span></span>
            <span class="text-[11px] text-purple-400 font-medium mt-1">64bit Float (DSP) ➡️ {earphoneBitDepth === 'F32' ? '32bit Float (OS 믹서)' : earphoneBitDepth}</span>
          </div>
        </div>
        <!-- Output 1: Earphone -->
        <div class="flex items-start gap-5 relative mb-7">
          <div class="w-7 h-7 rounded-full bg-white flex items-center justify-center z-10 shadow-lg shadow-white/20 shrink-0 text-black">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.121-2.121a3 3 0 01-4.243 0m4.243 0a8 8 0 010 11.314M4 12h8m-8 0a2 2 0 100 4 2 2 0 000-4z"></path></svg>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90">Primary Output</span>
            <span class="text-[11px] text-apple-blue font-medium mt-1">{formatDeviceName(selectedEarphone)}</span>
            <span class="text-[10px] text-white/40 mt-1">OS 믹서 / Core Audio</span>
          </div>
        </div>
      {/if}

      {#if target === 'speaker'}
        <!-- 우퍼 경로 (들여쓰기 제거, 메인 라인에 종속) -->
        <!-- Node 5: LPF (Speaker Path) -->
        <div class="flex items-start gap-5 relative mb-7">
          <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
            <div class="w-1.5 h-1.5 rounded-full bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.8)]"></div>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90 flex items-center gap-2">크로스오버 (Low Pass Filter) <span class="text-yellow-500">✨</span></span>
            <span class="text-[11px] text-yellow-500 font-medium mt-1">{lpfHz} Hz 미만 통과</span>
          </div>
        </div>

        <!-- Woofer Bit Depth Re-Conversion -->
        <div class="flex items-start gap-5 relative mb-7">
          <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/10 z-10 shrink-0 bg-[#0E0E10]">
            <div class="w-1.5 h-1.5 rounded-full bg-purple-400 shadow-[0_0_8px_rgba(192,132,252,0.8)]"></div>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90 flex items-center gap-2">비트심도 변경 <span class="text-purple-400">✨</span></span>
            <span class="text-[11px] text-purple-400 font-medium mt-1">64bit Float (DSP) ➡️ {speakerBitDepth === 'F32' ? '32bit Float (OS 믹서)' : speakerBitDepth}</span>
          </div>
        </div>

        <!-- Output 2: Speaker -->
        <div class="flex items-start gap-5 relative mb-2">
          <div class="w-7 h-7 rounded-full flex items-center justify-center border border-white/20 z-10 shadow-lg shadow-white/10 shrink-0 text-white bg-[#0E0E10]">
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.121-2.121a3 3 0 01-4.243 0m4.243 0a8 8 0 010 11.314M4 12h8m-8 0a2 2 0 100 4 2 2 0 000-4z"></path></svg>
          </div>
          <div class="flex flex-col pt-1">
            <span class="text-sm font-bold text-white/90">Sub Woofer Output</span>
            <span class="text-[11px] text-yellow-500 font-medium mt-1">{formatDeviceName(selectedSpeaker)}</span>
            <span class="text-[10px] text-white/40 mt-1">0ms Instant Bypass</span>
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>
