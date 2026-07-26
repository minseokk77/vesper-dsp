<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emit, listen } from '@tauri-apps/api/event';
  import { getCurrentWindow, getAllWindows } from '@tauri-apps/api/window';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { getVersion } from '@tauri-apps/api/app';
  import CustomSelect from '$lib/components/CustomSelect.svelte';

  // 설정 관련 상태
  let isSettingsOpen = false;
  let isLicenseOpen = false;
  let autoStartEnabled = false;
  let isWindowLocked = typeof window !== 'undefined' ? localStorage.getItem('ws_isWindowLocked') === 'true' : false;
  let currentVersion = '1.0.0';
  let updateStatus = '최신 버전 확인하기';
  let isChecking = false;
  let updateProgress = 0;
  let hasUpdate = false;
  let newVersion = '';
  let updateBody = '';

  let activePreset = 'Music';
  let toastMessage = '';
  let toastTimeout: any;

  function showToast(msg: string) {
    toastMessage = msg;
    clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => { toastMessage = ''; }, 3000);
  }

  function loadPreset(name: string) {
    activePreset = name;
    try {
      const dataStr = localStorage.getItem(`ws_preset_${name}`);
      if (dataStr) {
        const data = JSON.parse(dataStr);
        if (data.delayMs !== undefined) delayMs = data.delayMs;
        if (data.lpfHz !== undefined) lpfHz = data.lpfHz;
        if (data.lpfSlope !== undefined) lpfSlope = data.lpfSlope;
        if (data.isAsioMode !== undefined) isAsioMode = data.isAsioMode;
        if (data.selectedSource !== undefined) selectedSource = data.selectedSource;
        if (data.selectedEarphone !== undefined) selectedEarphone = data.selectedEarphone;
        if (data.selectedSpeaker !== undefined) selectedSpeaker = data.selectedSpeaker;
        if (data.headroomDb !== undefined) headroomDb = data.headroomDb;
        if (data.showClipping !== undefined) showClipping = data.showClipping;
        if (data.sampleRateStrategy !== undefined) sampleRateStrategy = data.sampleRateStrategy;
        if (data.sampleRateFilter !== undefined) sampleRateFilter = data.sampleRateFilter;
        if (data.dsdFilter !== undefined) dsdFilter = data.dsdFilter;
        if (data.dsdGain !== undefined) dsdGain = data.dsdGain;
        if (data.isEarphoneMuted !== undefined) {
          isEarphoneMuted = data.isEarphoneMuted;
          invoke('set_earphone_mute_cmd', { muted: isEarphoneMuted }).catch(console.error);
        }
        if (data.isSpeakerMuted !== undefined) {
          isSpeakerMuted = data.isSpeakerMuted;
          invoke('set_speaker_mute_cmd', { muted: isSpeakerMuted }).catch(console.error);
        }
        showToast(`${name} 프리셋을 불러왔습니다.`);
      }
    } catch(e) {
      console.error(e);
    }
  }

  function savePreset(name: string) {
    const data = { 
      delayMs, lpfHz, lpfSlope, isAsioMode,
      selectedSource, selectedEarphone, selectedSpeaker,
      headroomDb, showClipping, sampleRateStrategy, sampleRateFilter, dsdFilter, dsdGain,
      isEarphoneMuted, isSpeakerMuted
    };
    localStorage.setItem(`ws_preset_${name}`, JSON.stringify(data));
    activePreset = name;
    showToast(`${name} 프리셋이 저장되었습니다.`);
  }

  let devices: string[] = [];
  let selectedSource = '';
  let selectedEarphone = '';
  let selectedSpeaker = '';
  let delayMs = 0;
  let lpfHz = 80;
  let lpfSlope = 24; // 0 (Off), 12, 24
  let isSyncing = false;
  let isAsioMode = false; 

  // 고급 DSP 설정 상태 변수 (Roon Style)
  let headroomDb = -3.0;
  let showClipping = false;
  let sampleRateStrategy = '호환성 위주';
  let sampleRateFilter = '정확한 최소 단계';
  let dsdFilter = '권장함 (30kHz Low Pass Filter)';
  let dsdGain = '+6.0dB';

  let isSignalModalOpen = false;
  let signalModalTarget: 'earphone' | 'speaker' = 'earphone';

  function openSignalModal(target: 'earphone' | 'speaker') {
    signalModalTarget = target;
    isSignalModalOpen = true;
  }

  let isDspModalOpen = false;
  let dspModalTarget: 'earphone' | 'speaker' = 'earphone';

  // 클리핑 인디케이터 상태
  let isEarphoneClipping = false;
  let isSpeakerClipping = false;
  let earphoneClipTimer: any;
  let speakerClipTimer: any;

  // 개별 출력 음소거 상태
  let isEarphoneMuted = typeof window !== 'undefined' ? localStorage.getItem('ws_isEarphoneMuted') === 'true' : false;
  let isSpeakerMuted = typeof window !== 'undefined' ? (localStorage.getItem('ws_isSpeakerMuted') !== null ? localStorage.getItem('ws_isSpeakerMuted') === 'true' : true) : true;

  // FiiO K11 상태 변수
  let sampleRate = 44100;
  $: isFiioConnected = selectedEarphone.includes('FiiO') || selectedSpeaker.includes('FiiO');
  
  // 실시간 샘플 레이트 가져오기
  async function fetchSampleRate() {
    if (isFiioConnected) {
      const fiioName = selectedEarphone.includes('FiiO') ? selectedEarphone : selectedSpeaker;
      try {
        let rate = await invoke<number>('get_device_sample_rate', { deviceName: fiioName, isAsio: isAsioMode });
        sampleRate = rate;
      } catch (e) {
        console.error("Failed to fetch sample rate:", e);
      }
    }
  }

  // FiiO 장치 선택이나 ASIO 모드가 바뀔 때마다 실시간 업데이트
  $: if (isFiioConnected || isAsioMode !== undefined) {
    fetchSampleRate();
  }

  // FiiO 로고 컬러 감성 (Cyan <= 48kHz, Yellow > 48kHz, Green = DSD)
  $: fiioGlowColor = isFiioConnected 
    ? (sampleRate > 384000 ? 'rgba(34, 197, 94, 0.25)' 
      : sampleRate > 48000 ? 'rgba(234, 179, 8, 0.25)' 
      : 'rgba(6, 182, 212, 0.25)')
    : 'rgba(255, 255, 255, 0.0)'; 

  async function toggleSync() {
    isSyncing = !isSyncing;
    localStorage.setItem('ws_isSyncing', isSyncing ? 'true' : 'false');
    if (isSyncing) {
      try {
        let earTargetSr = null;
        let spkTargetSr = null;
        let earFilter = localStorage.getItem(`ws_srFilter_${selectedEarphone}`) || '정확한 최소 단계';
        let spkFilter = localStorage.getItem(`ws_srFilter_${selectedSpeaker}`) || '정확한 최소 단계';

        if (localStorage.getItem(`ws_srStrategy_${selectedEarphone}`) === '개별설정') {
          const tsr = localStorage.getItem(`ws_targetSr_${selectedEarphone}`);
          if (tsr) earTargetSr = Number(tsr);
        }
        if (localStorage.getItem(`ws_srStrategy_${selectedSpeaker}`) === '개별설정') {
          const tsr = localStorage.getItem(`ws_targetSr_${selectedSpeaker}`);
          if (tsr) spkTargetSr = Number(tsr);
        }

        await invoke('start_sync', {
          source: selectedSource,
          earphone: selectedEarphone,
          speaker: selectedSpeaker,
          delay: delayMs,
          lpfHz: lpfHz,
          lpfSlope: lpfSlope,
          isAsio: isAsioMode,
          headroomDb: headroomDb,
          earphoneTargetSr: earTargetSr,
          speakerTargetSr: spkTargetSr,
          earphoneFilter: earFilter,
          speakerFilter: spkFilter
        });
      } catch (e) {
        console.error(e);
        alert("백엔드 오류: " + e);
        isSyncing = false;
        localStorage.setItem('ws_isSyncing', 'false');
      }
    } else {
      await invoke('stop_sync');
    }
  }

  async function fetchDevices() {
    try {
      devices = await invoke('get_audio_devices', { isAsio: isAsioMode });
    } catch (e) {
      console.error(e);
      devices = ['Error fetching devices'];
    }
    
    // 저장된 기기가 현재 연결되어 있으면 복구, 없으면 스마트하게 기본값 찾기
    const savedSource = localStorage.getItem('ws_source') || '';
    const savedEarphone = localStorage.getItem('ws_earphone') || '';
    const savedSpeaker = localStorage.getItem('ws_speaker') || '';

    selectedSource = devices.includes(savedSource) ? savedSource : (devices.find(d => d.toLowerCase().includes('cable')) || devices[0] || '');
    selectedEarphone = devices.includes(savedEarphone) ? savedEarphone : (devices.find(d => d.toLowerCase().includes('fiio')) || devices[1] || '');
    selectedSpeaker = devices.includes(savedSpeaker) ? savedSpeaker : (devices.find(d => !d.toLowerCase().includes('fiio') && !d.toLowerCase().includes('cable')) || devices[2] || '');
  }

  function loadEarphoneProfile(earphone: string) {
    if (!earphone || typeof window === 'undefined') return;
    const h = localStorage.getItem(`ws_headroom_${earphone}`); if (h) headroomDb = Number(h);
    const c = localStorage.getItem(`ws_clipping_${earphone}`); if (c) showClipping = c === 'true';
    const ss = localStorage.getItem(`ws_srStrategy_${earphone}`); if (ss) sampleRateStrategy = ss;
    const sf = localStorage.getItem(`ws_srFilter_${earphone}`); if (sf) sampleRateFilter = sf;
    const df = localStorage.getItem(`ws_dsdFilter_${earphone}`); if (df) dsdFilter = df;
    const dg = localStorage.getItem(`ws_dsdGain_${earphone}`); if (dg) dsdGain = dg;
  }

  function loadSpeakerProfile(speaker: string) {
    if (!speaker || typeof window === 'undefined') return;
    const d = localStorage.getItem(`ws_delayMs_${speaker}`); if (d) delayMs = Number(d);
    const l = localStorage.getItem(`ws_lpfHz_${speaker}`); if (l) lpfHz = Number(l);
    const s = localStorage.getItem(`ws_lpfSlope_${speaker}`); if (s) lpfSlope = Number(s);
  }

  $: { if (selectedEarphone) loadEarphoneProfile(selectedEarphone); }
  $: { if (selectedSpeaker) loadSpeakerProfile(selectedSpeaker); }

  onMount(async () => {
    // 앱 전역 설정 (ASIO 모드)
    const savedAsio = localStorage.getItem('ws_isAsioMode');
    if (savedAsio !== null) isAsioMode = savedAsio === 'true';

    await fetchDevices();

    // 트레이 이벤트 등 리스닝
    listen('open-settings', () => {
      isSettingsOpen = true;
    });
    
    listen('toggle-sync', () => {
      toggleSync();
    });

    listen('load-preset', (event) => {
      if (typeof event.payload === 'string') {
        loadPreset(event.payload);
      }
    });

    currentVersion = await getVersion();
    autoStartEnabled = await isEnabled();

    // 백엔드 동기화 (기본 상태 전달)
    await invoke('set_earphone_mute_cmd', { muted: isEarphoneMuted });
    await invoke('set_speaker_mute_cmd', { muted: isSpeakerMuted });

    // 백엔드에서 윈도우 상태 검증 후 쏘므로 무조건 새 창 팝업
    listen('open-signal', async (e) => {
      const target = (e.payload || 'earphone') as 'earphone' | 'speaker';
      openSignalPath(target);
    });

    // 멀티 윈도우(설정 창)에서 값이 변경되었을 때 실시간 감지
    window.addEventListener('storage', (e) => {
      const cur = selectedEarphone;
      if (!cur) return;
      if (e.key === `ws_headroom_${cur}`) headroomDb = Number(e.newValue);
      if (e.key === `ws_clipping_${cur}`) showClipping = e.newValue === 'true';
      if (e.key === `ws_srStrategy_${cur}`) sampleRateStrategy = e.newValue || '호환성 위주';
      if (e.key === `ws_srFilter_${cur}`) sampleRateFilter = e.newValue || '정확한 최소 단계';
      if (e.key === `ws_dsdFilter_${cur}`) dsdFilter = e.newValue || '권장함 (30kHz Low Pass Filter)';
      if (e.key === `ws_dsdGain_${cur}`) dsdGain = e.newValue || '+6.0dB';
    });

    // 자동 시작 상태 복구 (이전에 켜둔 상태였다면 앱 부팅 시 자동 시작)
    if (localStorage.getItem('ws_isSyncing') === 'true') {
      setTimeout(() => {
        if (!isSyncing) toggleSync();
      }, 500);
    }

    // 클리핑 이벤트 수신
    listen('clipping-detected', (event) => {
      if (!showClipping) return;
      const target = event.payload as string;
      if (target === 'earphone') {
        isEarphoneClipping = true;
        clearTimeout(earphoneClipTimer);
        earphoneClipTimer = setTimeout(() => { isEarphoneClipping = false; }, 500);
      } else if (target === 'speaker') {
        isSpeakerClipping = true;
        clearTimeout(speakerClipTimer);
        speakerClipTimer = setTimeout(() => { isSpeakerClipping = false; }, 500);
      }
    });
  });

  function openDspModal(target: 'earphone' | 'speaker') {
    dspModalTarget = target;
    isDspModalOpen = true;
  }

  async function openSignalPath(target: 'earphone' | 'speaker') {
    try {
      const label = `signal_path`;
      
      const windows = await getAllWindows();
      const existing = windows.find(w => w.label === label);
      
      if (existing) {
        await existing.unminimize();
        await existing.show();
        await existing.setFocus();
        await emit('change-signal-target', { target });
        return;
      }

      const spWindow = new WebviewWindow(label, {
        url: `/signal?target=${target}`,
        title: 'Signal Path',
        width: 350,
        height: 650,
        resizable: false, // 시그널 패스 창 크기 고정
        center: true,
        decorations: false,
        transparent: true
      });
    } catch(err) {
      alert('Tauri Window JS 에러: ' + err);
    }
  }

  async function openSettings() {
    isSettingsOpen = true;
  }

  async function toggleAutoStart() {
    try {
      if (autoStartEnabled) {
        await disable();
        autoStartEnabled = false;
      } else {
        await enable();
        autoStartEnabled = true;
      }
    } catch (e) {
      console.error('Autostart Error:', e);
      alert('자동 시작 설정을 변경하는 중 오류가 발생했습니다.');
    }
  }

  function toggleWindowLock() {
    isWindowLocked = !isWindowLocked;
    localStorage.setItem('ws_isWindowLocked', isWindowLocked.toString());
  }

  async function checkUpdate() {
    if (isChecking) return;
    isChecking = true;
    updateStatus = '업데이트 확인 중...';
    try {
      const update = await check();
      if (update) {
        hasUpdate = true;
        newVersion = update.version;
        updateBody = update.body || '새로운 기능 및 버그 수정이 포함되어 있습니다.';
        updateStatus = '업데이트 가능';
      } else {
        hasUpdate = false;
        updateStatus = '최신 버전을 사용 중입니다.';
      }
    } catch (e) {
      console.error('Update Check Error:', e);
      updateStatus = '확인 실패. 네트워크를 점검하세요.';
    }
    isChecking = false;
  }

  async function installUpdate() {
    isChecking = true;
    updateStatus = '다운로드 및 설치 중...';
    try {
      const update = await check();
      if (update) {
        let downloaded = 0;
        let contentLength = 0;
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case 'Started':
              contentLength = event.data.contentLength || 0;
              updateStatus = '다운로드 시작됨...';
              break;
            case 'Progress':
              downloaded += event.data.chunkLength;
              if (contentLength > 0) {
                updateProgress = Math.round((downloaded / contentLength) * 100);
                updateStatus = `다운로드 중... ${updateProgress}%`;
              } else {
                updateStatus = '다운로드 중...';
              }
              break;
            case 'Finished':
              updateStatus = '설치 완료. 재시작 중...';
              break;
          }
        });
        await relaunch();
      }
    } catch (e) {
      console.error('Install Error:', e);
      updateStatus = '설치 실패. 다시 시도해주세요.';
    }
    isChecking = false;
  }

  // Mute 토글 함수
  async function toggleEarphoneMute() {
    isEarphoneMuted = !isEarphoneMuted;
    await invoke('set_earphone_mute_cmd', { muted: isEarphoneMuted });
  }

  async function toggleSpeakerMute() {
    isSpeakerMuted = !isSpeakerMuted;
    await invoke('set_speaker_mute_cmd', { muted: isSpeakerMuted });
  }

  // 상태가 바뀔 때마다 자동으로 백그라운드 저장
  $: {
    if (typeof window !== 'undefined' && selectedSpeaker) {
      localStorage.setItem(`ws_delayMs_${selectedSpeaker}`, delayMs.toString());
      localStorage.setItem(`ws_lpfHz_${selectedSpeaker}`, lpfHz.toString());
      localStorage.setItem(`ws_lpfSlope_${selectedSpeaker}`, lpfSlope.toString());
      localStorage.setItem('ws_isEarphoneMuted', isEarphoneMuted.toString());
      localStorage.setItem('ws_isSpeakerMuted', isSpeakerMuted.toString());
      localStorage.setItem('ws_speaker', selectedSpeaker);
      localStorage.setItem('ws_current_speaker', selectedSpeaker);
      emit('update-signal-path');
    }
  }

  $: {
    if (typeof window !== 'undefined' && selectedEarphone) {
      localStorage.setItem(`ws_headroom_${selectedEarphone}`, headroomDb.toString());
      localStorage.setItem(`ws_clipping_${selectedEarphone}`, showClipping.toString());
      localStorage.setItem(`ws_srStrategy_${selectedEarphone}`, sampleRateStrategy);
      localStorage.setItem(`ws_srFilter_${selectedEarphone}`, sampleRateFilter);
      localStorage.setItem(`ws_dsdFilter_${selectedEarphone}`, dsdFilter);
      localStorage.setItem(`ws_dsdGain_${selectedEarphone}`, dsdGain);
      localStorage.setItem('ws_earphone', selectedEarphone);
      // DSP 창과 통신하기 위한 현재 선택 이어폰 브로드캐스트
      localStorage.setItem('ws_current_earphone', selectedEarphone);
      emit('update-signal-path');
    }
  }

  $: {
    if (typeof window !== 'undefined' && selectedSource) {
      localStorage.setItem('ws_isAsioMode', isAsioMode.toString());
      localStorage.setItem('ws_source', selectedSource);
      emit('update-signal-path');
    }
  }

  $: {
    if (isAsioMode !== undefined && typeof window !== 'undefined') {
      fetchDevices();
    }
  }
</script>

<div class="relative w-screen h-screen transition-all duration-1000 ease-in-out"
     style="box-shadow: 0 0 60px {fiioGlowColor};">
  
  <div class="liquid-glass w-full h-full flex flex-col relative overflow-hidden">
    
    <!-- 메인 컨텐츠 영역 (스크롤 가능) -->
    <div class="flex-1 flex flex-col gap-5 p-5 pt-5 overflow-y-auto overflow-x-hidden">
      <!-- 헤더부: 타이틀 및 API 토글 (통합된 드래그 & 컨트롤바) -->
      <div 
        class="flex items-start justify-between z-10 pb-2 {isWindowLocked ? '' : 'cursor-grab active:cursor-grabbing'}"
        on:mousedown={() => { if (!isWindowLocked) getCurrentWindow().startDragging(); }}
      >
        <!-- 좌측: 메인 타이틀 및 상태 (드래그 반응) -->
        <div class="pointer-events-none">
          <h1 class="text-2xl font-bold tracking-tight text-white/90">Vesper <span class="text-white/30 font-normal mx-0.5">|</span> Woofer</h1>
          <div class="flex items-center gap-2 mt-1">
            <div class="h-2 w-2 rounded-full transition-all duration-500 {isSyncing ? 'bg-green-500 shadow-[0_0_10px_rgba(34,197,94,0.8)]' : 'bg-red-500/80'}"></div>
            <span class="text-xs font-medium text-white/50 tracking-wider uppercase">
              {isSyncing ? 'Active' : 'Standby'}
            </span>
          </div>
        </div>

        <!-- 우측: 컨트롤 영역 (드래그 이벤트 전파 방지) -->
        <div class="flex flex-col items-end gap-3" on:mousedown|stopPropagation>
          <!-- 윈도우 조작 버튼 -->
          <div class="flex items-center gap-3">
            <!-- 설정 버튼 -->
            <button on:click={openSettings} class="w-4 h-4 rounded-full bg-transparent flex items-center justify-center text-white/40 hover:text-white/80 transition-colors cursor-pointer mr-1" aria-label="Settings" title="환경설정">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
            </button>
            <button on:click={() => getCurrentWindow().minimize()} class="w-3 h-3 rounded-full bg-white/20 hover:bg-yellow-500 transition-colors" aria-label="Minimize"></button>
            <button on:click={() => getCurrentWindow().hide()} class="w-3 h-3 rounded-full bg-white/20 hover:bg-red-500 transition-colors" aria-label="Hide to Tray"></button>
          </div>

          <!-- macOS 스타일 Segmented Control -->
          <div class="flex bg-black/40 p-1 rounded-xl shadow-inner border border-white/5 mt-1">
            <button 
              class="px-4 py-1.5 text-xs font-semibold rounded-lg transition-all duration-300 {!isAsioMode ? 'bg-white/15 text-white shadow-sm' : 'text-white/40 hover:text-white/70'}"
              on:click={() => isAsioMode = false}
            >WASAPI</button>
            <button 
              class="px-4 py-1.5 text-xs font-semibold rounded-lg transition-all duration-300 {isAsioMode ? 'bg-white/15 text-white shadow-sm' : 'text-white/40 hover:text-white/70'}"
              on:click={() => isAsioMode = true}
            >ASIO</button>
          </div>
        </div>
      </div>

    <!-- FiiO 상태 바 (연결되었을 때만 노출) -->
    {#if isFiioConnected}
      <div class="flex items-center justify-between pb-3 border-b border-white/5">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-full flex items-center justify-center bg-black/30">
            <span class="w-3 h-3 rounded-full" style="background-color: {fiioGlowColor.replace('0.25', '1')}; box-shadow: 0 0 12px {fiioGlowColor.replace('0.25', '1')}"></span>
          </div>
          <div>
            <p class="text-xs font-semibold text-white/90">FiiO K11 DAC</p>
            <p class="text-[10px] text-white/50 tracking-wider">Bit-Perfect Mode</p>
          </div>
        </div>
        <div class="text-right">
          <p class="text-sm font-bold tracking-tight text-white/90">{sampleRate / 1000} kHz</p>
          <p class="text-[10px] text-white/50">Sample Rate</p>
        </div>
      </div>
    {/if}

    <!-- 설정 패널들 -->
    <div class="flex flex-col gap-3 mt-1">
      
      <!-- Presets -->
      <div class="flex items-center justify-between bg-black/20 p-1 rounded-xl border border-white/5 mb-1 shadow-inner">
        <div class="flex gap-1">
          <button class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {activePreset === 'Movie' ? 'bg-white/10 text-white shadow-sm' : 'text-white/40 hover:text-white/70'}" on:click={() => loadPreset('Movie')}>🎬 Movie</button>
          <button class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {activePreset === 'Music' ? 'bg-white/10 text-white shadow-sm' : 'text-white/40 hover:text-white/70'}" on:click={() => loadPreset('Music')}>🎵 Music</button>
          <button class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-all {activePreset === 'Gaming' ? 'bg-white/10 text-white shadow-sm' : 'text-white/40 hover:text-white/70'}" on:click={() => loadPreset('Gaming')}>🎮 Gaming</button>
        </div>
        <button class="px-3 py-1.5 mr-1 rounded-lg text-[10px] font-bold tracking-widest uppercase text-apple-blue/70 hover:text-apple-blue hover:bg-apple-blue/10 transition-colors" on:click={() => savePreset(activePreset)} title="현재 설정을 덮어쓰기">
          Save
        </button>
      </div>
      
      <!-- Audio Source -->
      <div class="flex flex-col gap-2 relative group">
        <label class="text-[11px] font-semibold tracking-wider text-white/50 uppercase pl-1">Audio Source (Virtual Cable)</label>
        <CustomSelect 
          bind:value={selectedSource}
          options={devices.map(d => ({ value: d, label: d }))}
        />
      </div>

      <!-- Earphone -->
      <div class="flex flex-col gap-2 border-t border-white/5 pt-2 p-1.5 -mx-1.5 rounded-xl relative group transition-all duration-300 {isEarphoneClipping ? 'bg-red-500/20 ring-2 ring-red-500 shadow-[0_0_30px_rgba(239,68,68,0.3)]' : ''}">
        <div class="flex justify-between items-center pr-1">
          <label class="text-[11px] font-semibold tracking-wider text-white/50 uppercase pl-1">Primary Earphones</label>
          <div class="flex items-center gap-2">
            <!-- 시그널 패스 버튼 -->
            <button on:click={() => openSignalModal('earphone')} class="flex items-center justify-center w-5 h-5 rounded-full bg-white/5 hover:bg-white/20 transition-colors group" title="시그널 패스 보기">
              <svg class="w-3 h-3 text-white/40 group-hover:text-apple-blue transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012-2h-2a2 2 0 01-2-2z"></path></svg>
            </button>
            <!-- 고급 설정 버튼 -->
            <button on:click={() => openDspModal('earphone')} class="flex items-center justify-center w-5 h-5 rounded-full bg-white/5 hover:bg-white/20 transition-colors group" title="고급 DSP 설정">
              <svg class="w-3 h-3 text-white/40 group-hover:text-white/80 group-hover:animate-spin-slow transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
            </button>
            <!-- 전원(Mute) 토글 버튼 -->
            <button on:click={toggleEarphoneMute} class="flex items-center justify-center w-5 h-5 rounded-full transition-colors group {isEarphoneMuted ? 'bg-red-500/20 hover:bg-red-500/40' : 'bg-green-500/20 hover:bg-green-500/40'}" title="출력 활성화/비활성화">
              <svg class="w-3 h-3 transition-colors {isEarphoneMuted ? 'text-red-500/80 group-hover:text-red-400' : 'text-green-500/80 group-hover:text-green-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.36 6.64a9 9 0 11-12.73 0M12 2v10"></path></svg>
            </button>
          </div>
        </div>
        <CustomSelect 
          bind:value={selectedEarphone}
          options={devices.map(d => ({ value: d, label: d }))}
        />
      </div>

      <!-- Speaker -->
      <div class="flex flex-col gap-2 border-t border-white/5 pt-2 p-1.5 -mx-1.5 rounded-xl relative group transition-all duration-300 {isSpeakerClipping ? 'bg-red-500/20 ring-2 ring-red-500 shadow-[0_0_30px_rgba(239,68,68,0.3)]' : ''}">
        <div class="flex justify-between items-center pr-1">
          <label class="text-[11px] font-semibold tracking-wider text-white/50 uppercase pl-1">Sub Woofer</label>
          <div class="flex items-center gap-2">
            <!-- 시그널 패스 버튼 -->
            <button on:click={() => openSignalModal('speaker')} class="flex items-center justify-center w-5 h-5 rounded-full bg-white/5 hover:bg-white/20 transition-colors group" title="시그널 패스 보기">
              <svg class="w-3 h-3 text-white/40 group-hover:text-yellow-500 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path></svg>
            </button>
            <!-- 고급 설정 버튼 -->
            <button on:click={() => openDspModal('speaker')} class="flex items-center justify-center w-5 h-5 rounded-full bg-white/5 hover:bg-white/20 transition-colors group" title="고급 DSP 설정">
              <svg class="w-3 h-3 text-white/40 group-hover:text-white/80 group-hover:animate-spin-slow transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
            </button>
            <!-- 전원(Mute) 토글 버튼 -->
            <button on:click={toggleSpeakerMute} class="flex items-center justify-center w-5 h-5 rounded-full transition-colors group {isSpeakerMuted ? 'bg-red-500/20 hover:bg-red-500/40' : 'bg-green-500/20 hover:bg-green-500/40'}" title="출력 활성화/비활성화">
              <svg class="w-3 h-3 transition-colors {isSpeakerMuted ? 'text-red-500/80 group-hover:text-red-400' : 'text-green-500/80 group-hover:text-green-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.36 6.64a9 9 0 11-12.73 0M12 2v10"></path></svg>
            </button>
          </div>
        </div>
        <CustomSelect 
          bind:value={selectedSpeaker}
          options={devices.map(d => ({ value: d, label: d }))}
        />
      </div>
    </div>

    <!-- 딜레이 슬라이더 -->
    <div class="flex flex-col gap-2 border-t border-white/5 pt-2">
      <div class="flex justify-between items-end">
        <label class="text-[11px] font-semibold tracking-wider text-white/50 uppercase">Phase Delay</label>
        <span class="text-xl font-bold tracking-tighter text-white/90">{delayMs}<span class="text-xs text-white/40 ml-1 font-medium">ms</span></span>
      </div>
      <input 
        type="range" 
        min="0" max="1000" 
        bind:value={delayMs} 
        class="apple-slider" 
      />
    </div>

    <!-- Subwoofer Crossover Settings -->
    <div class="flex flex-col gap-3 border-t border-white/5 pt-2">
      <div class="flex justify-between items-end">
        <h3 class="text-xs font-bold tracking-widest text-white/70 uppercase">Crossover (Sub Woofer)</h3>
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-white/50 uppercase tracking-wider">Slope</span>
          <div class="w-[150px]">
            <CustomSelect 
              bind:value={lpfSlope} 
              align="right"
              bgClass="bg-white/5 hover:bg-white/10"
              options={[
                { value: 0, label: 'Off' },
                { value: 12, label: '12 dB/Octave' },
                { value: 24, label: '24 dB/Octave' }
              ]}
            />
          </div>
        </div>
      </div>
      
      <div class="flex flex-col gap-2">
        <div class="flex justify-between items-center text-sm font-medium">
          <span class="text-white/60">Low Pass Filter</span>
          <span class="text-apple-blue font-bold">{lpfHz} Hz</span>
        </div>
        <input 
          type="range" 
          min="40" 
          max="200" 
          step="5"
          bind:value={lpfHz}
          class="w-full h-1.5 bg-white/10 rounded-full appearance-none cursor-pointer
                 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 
                 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:shadow-lg
                 hover:[&::-webkit-slider-thumb]:scale-110 transition-all"
        />
        <p class="text-[10px] text-white/40 mt-1">지정한 주파수보다 낮은 저음역대만 우퍼로 통과시킵니다.</p>
      </div>
    </div>

    <!-- 재생/중지 토글 버튼 -->
    <button 
      class="w-full mt-auto mb-2 py-3 rounded-2xl font-bold text-sm tracking-wide uppercase transition-all duration-300 active:scale-[0.97]
             {isSyncing 
               ? 'bg-red-500/20 text-red-400 border border-red-500/30 hover:bg-red-500/30 shadow-[0_0_20px_rgba(239,68,68,0.15)]' 
               : 'bg-white text-black hover:bg-gray-200 shadow-lg'}"
      on:click={toggleSync}
    >
      {isSyncing ? 'Stop Engine' : 'Engage Sync'}
    </button>
    </div> <!-- 메인 컨텐츠 영역 끝 -->

    <!-- Toast Notification -->
    {#if toastMessage}
      <div class="absolute bottom-20 left-1/2 -translate-x-1/2 z-[100] px-5 py-2.5 bg-black/80 backdrop-blur-xl border border-white/10 rounded-full shadow-[0_10px_40px_rgba(0,0,0,0.5)] text-white/90 text-[13px] font-semibold tracking-wide pointer-events-none animate-in slide-in-from-bottom-5 fade-in duration-300">
        {toastMessage}
      </div>
    {/if}

    <!-- Settings Modal -->
    {#if isSettingsOpen}
    <div class="absolute inset-0 z-50 flex items-center justify-center p-5 bg-black/60 backdrop-blur-md animate-in fade-in duration-200" on:click|self={() => isSettingsOpen = false}>
      <div class="w-full max-w-sm bg-[#0E0E10]/95 border border-white/10 rounded-2xl flex flex-col shadow-[0_8px_32px_rgba(0,0,0,0.8)] overflow-hidden">
        <!-- Header -->
        <div class="flex items-center justify-between p-4 border-b border-white/5 bg-white/5">
          <h2 class="text-sm font-bold tracking-tight text-white/90">환경설정 (Settings)</h2>
          <button on:click={() => isSettingsOpen = false} class="w-6 h-6 rounded-full bg-white/10 flex items-center justify-center hover:bg-white/20 transition-colors" aria-label="Close">
            <svg class="w-3 h-3 text-white/70" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
          </button>
        </div>
        
        <div class="p-5 flex-1 space-y-6">
          <!-- System Settings -->
          <div class="space-y-3">
            <h3 class="text-[10px] font-bold tracking-widest text-white/50 uppercase">System</h3>
            <div class="flex justify-between items-center bg-black/30 p-4 rounded-xl border border-white/5">
              <div>
                <p class="text-xs font-semibold text-white/90">Windows 시작 시 자동 실행</p>
                <p class="text-[9px] text-white/50 mt-1">부팅 시 백그라운드로 자동 실행</p>
              </div>
              <button class="w-10 h-5 rounded-full transition-colors {autoStartEnabled ? 'bg-green-500' : 'bg-white/20'} relative" on:click={toggleAutoStart}>
                <div class="absolute w-4 h-4 bg-white rounded-full top-[2px] transition-transform {autoStartEnabled ? 'translate-x-5' : 'translate-x-[2px]'} shadow-sm"></div>
              </button>
            </div>
            
            <div class="flex justify-between items-center bg-black/30 p-4 rounded-xl border border-white/5 mt-2">
              <div>
                <p class="text-xs font-semibold text-white/90">창 위치 잠금 (이동 방지)</p>
                <p class="text-[9px] text-white/50 mt-1">원하는 곳에 둔 후 켜두면 항상 그 위치에 고정됨</p>
              </div>
              <button class="w-10 h-5 rounded-full transition-colors {isWindowLocked ? 'bg-apple-blue' : 'bg-white/20'} relative" on:click={toggleWindowLock}>
                <div class="absolute w-4 h-4 bg-white rounded-full top-[2px] transition-transform {isWindowLocked ? 'translate-x-5' : 'translate-x-[2px]'} shadow-sm"></div>
              </button>
            </div>
          </div>

          <!-- Update Settings -->
          <div class="space-y-3">
            <h3 class="text-[10px] font-bold tracking-widest text-white/50 uppercase">Updates</h3>
            <div class="flex flex-col bg-black/30 p-4 rounded-xl border border-white/5 gap-3">
              <div class="flex justify-between items-center">
                <div>
                  <p class="text-xs font-semibold text-white/90">현재 버전</p>
                  <p class="text-[10px] text-white/50 font-mono mt-0.5">v{currentVersion}</p>
                </div>
                
                {#if !hasUpdate}
                  <button class="px-3 py-1.5 text-[10px] font-semibold rounded-lg bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 transition-colors flex items-center gap-1.5 {isChecking ? 'opacity-50 cursor-not-allowed' : ''}" on:click={checkUpdate} disabled={isChecking}>
                    {#if isChecking}
                      <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
                    {/if}
                    {updateStatus}
                  </button>
                {/if}
              </div>

              {#if hasUpdate}
                <div class="border-t border-white/5 pt-3 mt-1">
                  <p class="text-xs font-bold text-green-400 mb-1">새 버전 발견: v{newVersion}</p>
                  <p class="text-[10px] text-white/60 mb-3 leading-relaxed break-keep">{updateBody}</p>

                  {#if updateProgress > 0}
                    <div class="w-full bg-white/10 rounded-full h-1.5 mb-2 overflow-hidden">
                      <div class="bg-green-500 h-1.5 rounded-full transition-all duration-300" style="width: {updateProgress}%"></div>
                    </div>
                  {/if}

                  <button class="w-full py-2 text-[11px] font-bold rounded-lg bg-green-500/20 text-green-400 hover:bg-green-500/30 transition-colors {isChecking ? 'opacity-50 cursor-not-allowed' : ''}" on:click={installUpdate} disabled={isChecking}>
                    {updateStatus === '업데이트 가능' ? '지금 다운로드 및 다시 시작' : updateStatus}
                  </button>
                </div>
              {/if}
            </div>
          </div>

          <div class="space-y-3">
            <h3 class="text-[10px] font-bold tracking-widest text-white/50 uppercase">About</h3>
            <div class="flex items-center justify-between bg-black/30 p-4 rounded-xl border border-white/5">
              <div>
                <p class="text-xs font-semibold text-white/90">오픈소스 고지</p>
                <p class="text-[9px] text-white/50 mt-1">Vesper Woofer에 사용된 오픈소스 라이선스를 확인합니다.</p>
              </div>
              <button
                class="w-8 h-8 rounded-full flex items-center justify-center bg-white/5 border border-white/10 hover:bg-white/10 hover:border-white/20 transition-colors group"
                on:click={() => isLicenseOpen = true}
                aria-label="오픈소스 고지 보기"
                title="고지 보기"
              >
                <svg class="w-4 h-4 text-white/50 group-hover:text-white transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    {/if}
  </div>
</div>

{#if isLicenseOpen}
  <div class="fixed inset-0 z-[60] flex items-center justify-center p-5 bg-black/70 backdrop-blur-md" on:click|self={() => isLicenseOpen = false}>
    <div class="w-full max-w-sm max-h-[80vh] bg-[#0E0E10] border border-white/10 rounded-2xl shadow-[0_8px_32px_rgba(0,0,0,0.8)] flex flex-col overflow-hidden">
      <div class="flex items-center justify-between p-4 border-b border-white/5 bg-white/5">
        <div>
          <h2 class="text-sm font-bold text-white/90">오픈소스 고지</h2>
          <p class="text-[10px] text-white/45 mt-1">Vesper Woofer</p>
        </div>
        <button on:click={() => isLicenseOpen = false} aria-label="오픈소스 고지 닫기" class="w-7 h-7 rounded-full bg-white/10 flex items-center justify-center hover:bg-white/20 transition-colors">
          <svg class="w-3 h-3 text-white/70" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>
      <div class="p-5 space-y-3 overflow-y-auto">
        <p class="text-[11px] text-white/60 leading-relaxed">Vesper Woofer는 OPRA/AutoEQ 데이터를 사용합니다.</p>
        <div class="rounded-xl bg-black/30 border border-white/5 p-3">
          <p class="text-xs font-semibold text-white/90">OPRA / AutoEQ</p>
          <p class="text-[10px] text-white/50 mt-1">헤드폰·이어폰 EQ 프로파일 데이터 · MIT License</p>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- 시그널 패스 내부 모달 -->
{#if isSignalModalOpen}
  <div class="absolute inset-0 z-50 flex items-center justify-center p-6 bg-black/60 backdrop-blur-md animate-in fade-in duration-200" on:mousedown|self={() => isSignalModalOpen = false}>
    <div class="w-full h-full max-w-sm max-h-[660px] bg-white/10 backdrop-blur-xl border border-white/20 rounded-3xl shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
      
      <!-- 헤더 영역 -->
      <div class="flex items-center justify-between p-4 border-b border-white/10 bg-black/20">
        <h2 class="text-lg font-bold text-white tracking-wide">
          {signalModalTarget === 'earphone' ? 'Earphone Signal Path' : 'Woofer Signal Path'}
        </h2>
        <button on:click={() => isSignalModalOpen = false} class="w-8 h-8 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>

      <!-- Iframe 컨텐츠 영역 -->
      <div class="flex-1 w-full bg-transparent">
        <iframe 
          src="/signal?target={signalModalTarget}&is_modal=true" 
          class="w-full h-full border-none bg-transparent"
          title="Signal Path Viewer"
        ></iframe>
      </div>
    </div>
  </div>
{/if}

<!-- 고급 설정 내부 모달 -->
{#if isDspModalOpen}
  <div class="absolute inset-0 z-50 flex items-center justify-center p-6 bg-black/60 backdrop-blur-md animate-in fade-in duration-200" on:mousedown|self={() => isDspModalOpen = false}>
    <div class="w-full h-full max-w-lg max-h-[700px] bg-white/10 backdrop-blur-xl border border-white/20 rounded-3xl shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
      
      <!-- 헤더 영역 -->
      <div class="flex items-center justify-between p-4 border-b border-white/10 bg-black/20">
        <h2 class="text-lg font-bold text-white tracking-wide">
          {dspModalTarget === 'earphone' ? '고급 설정 (이어폰)' : '고급 설정 (서브 우퍼)'}
        </h2>
        <button on:click={() => isDspModalOpen = false} class="w-8 h-8 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>

      <!-- Iframe 컨텐츠 영역 -->
      <div class="flex-1 w-full bg-transparent">
        <iframe 
          src="/dsp?target={dspModalTarget}&is_modal=true" 
          class="w-full h-full border-none bg-transparent"
          title="DSP Settings Viewer"
        ></iframe>
      </div>
    </div>
  </div>
{/if}
