<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emit, listen } from '@tauri-apps/api/event';
  import { getCurrentWindow, PhysicalPosition } from '@tauri-apps/api/window';
  import { getVersion } from '@tauri-apps/api/app';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check as checkUpdate } from '@tauri-apps/plugin-updater';
  import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
  import CustomSelect from '$lib/components/CustomSelect.svelte';

  // 상태 변수
  let isWindowLocked = typeof window !== 'undefined' ? localStorage.getItem('vesper_dsp_window_locked') === 'true' : false;
  let isRunning = false;
  let isStarting = false;
  let isMuted = false;
  let sourceDevices: string[] = [];
  let outputDevices: string[] = [];
  
  let source = '';
  let output = '';
  let targetRate: number | null = null;
  let strategy = '호환성 위주';
  type ResamplerFilter = 'linear_precise' | 'minimum_phase' | 'linear_smooth' | 'phase_smooth';
  let filterType: ResamplerFilter = 'minimum_phase';
  let dsdFilter = '권장함 (30kHz Low Pass Filter)';
  let dsdGain = '+6.0dB';
  
  let headroomDb = -3.0;
  let showClipping = false;
  let isClipping = false;
  let clipTimer: ReturnType<typeof setTimeout>;

  // 모달 제어
  let isSignalModalOpen = false;
  let signalModalHeight = 500;
  let sourceMenuOpen = false;
  let outputMenuOpen = false;
  let strategyMenuOpen = false;
  let filterMenuOpen = false;
  let dsdFilterMenuOpen = false;
  let dsdGainMenuOpen = false;

  // AutoEQ 상태
  let autoEqEnabled = false;
  let autoEqQuery = '';
  let autoEqLoading = false;
  let autoEqMode: 'opra' | 'spinorama' = 'opra';
  interface AutoEqResult { vendor: string; product: string; mode: 'opra' | 'spinorama'; }
  interface AutoEqTreeNode { path: string; }
  interface EqBandPayload { filter_type: string; frequency: number; gain_db: number; q: number; }
  interface SavedAutoEq { vendor: string; product: string; mode: 'opra' | 'spinorama'; enabled: boolean; preamp_gain?: number; bands?: EqBandPayload[]; }
  interface StreamInfo {
    source_sample_rate: number;
    source_bit_depth: string;
    source_channels: number;
    output_sample_rate: number;
    output_bit_depth: string;
    output_channels: number;
  }
  interface EngineStatus { running: boolean; }
  interface EngineError { stage: string; message: string; }

  import spinoramaData from '$lib/spinorama_index.json';

  let autoEqResults: AutoEqResult[] = [];
  let autoEqTree: AutoEqTreeNode[] = [];
  let autoEqProduct = '';
  
  $: eqEnabled = autoEqEnabled;

  let message = '';
  let settingsRestored = false;

  // Settings & System
  let showSettings = false;
  let showLicense = false;
  let showThirdPartyLicense = false;
  let autoStartEnabled = false;
  let currentVersion = '';
  let updateStatus = '업데이트 확인';
  let isCheckingUpdate = false;
  let hasUpdate = false;
  let newVersion = '';
  let updateBody = '';
  type WindowPosition = { x: number; y: number };
  const windowLockPositionKey = 'vesper_dsp_window_lock_position';
  let lockedWindowPosition: WindowPosition | null = null;
  let isRestoringLockedPosition = false;
  let unlistenWindowMoved: (() => void) | undefined;
  let eventUnlisteners: Array<() => void> = [];
  const supportedRates = [44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000, 705600, 768000];

  const rateOptions = () => [
    { value: null, label: '기기 최고 지원 레이트' },
    ...supportedRates.map((rate) => ({ value: rate, label: `${(rate / 1000).toFixed(rate % 1000 ? 1 : 0)} kHz` }))
  ];
  const strategyOptions = [
    { value: '호환성 위주', label: '호환성 위주' },
    { value: '최대 PCM 비율', label: '최대 PCM 비율' },
    { value: '최대 PCM 레이트 (2의 제곱)', label: '최대 PCM 레이트 (2의 제곱)' },
    { value: '개별설정', label: '개별설정' }
  ];
  const filterOptions = [
    { value: 'linear_precise', label: '정밀한, 선형 위상' },
    { value: 'minimum_phase', label: '정확한 최소 단계 (Minimum Phase)' },
    { value: 'linear_smooth', label: '부드러움, 리니어 위상' },
    { value: 'phase_smooth', label: '부드러움, 위상 변화 최소화' }
  ];
  const dsdFilterOptions = [
    { value: '보편적인 (24kHz Low Pass Filter)', label: '보편적인 (24kHz Low Pass Filter)' },
    { value: '권장함 (30kHz Low Pass Filter)', label: '권장함 (30kHz Low Pass Filter)' },
    { value: '허용적인 (50kHz Low Pass Filter)', label: '허용적인 (50kHz Low Pass Filter)' },
    { value: '필터링되지 않음 (주의해서 사용)', label: '필터링되지 않음 (주의해서 사용)' }
  ];
  const dsdGainOptions = [
    { value: '+0.0dB', label: '+0.0dB' },
    { value: '+1.0dB', label: '+1.0dB' },
    { value: '+2.0dB', label: '+2.0dB' },
    { value: '+3.0dB', label: '+3.0dB' },
    { value: '+4.0dB', label: '+4.0dB' },
    { value: '+5.0dB', label: '+5.0dB' },
    { value: '+6.0dB', label: '+6.0dB' }
  ];

  function save() {
    localStorage.setItem('vesper_dsp_source', source);
    localStorage.setItem('vesper_dsp_output', output);
    localStorage.setItem('vesper_dsp_target_rate', targetRate ? targetRate.toString() : '');
    localStorage.setItem('vesper_dsp_strategy', strategy);
    localStorage.setItem('vesper_dsp_filter', filterType);
    localStorage.setItem('vesper_dsp_dsd_filter', dsdFilter);
    localStorage.setItem('vesper_dsp_dsd_gain', dsdGain);
    localStorage.setItem('vesper_dsp_headroom', headroomDb.toString());
    localStorage.setItem('vesper_dsp_show_clipping', String(showClipping));
    emit('update-signal-path');
  }

  async function fetchDevices() {
    try {
      [sourceDevices, outputDevices] = await Promise.all([
        invoke<string[]>('get_source_devices', { isAsio: false }),
        invoke<string[]>('get_output_devices', { isAsio: false })
      ]);
    } catch (e) {
      console.error(e);
      sourceDevices = ['장치 오류'];
      outputDevices = ['장치 오류'];
    }
    const savedSource = localStorage.getItem('vesper_dsp_source') || '';
    const savedOutput = localStorage.getItem('vesper_dsp_output') || '';
    source = sourceDevices.includes(savedSource) ? savedSource : (sourceDevices.find(d => d.toLowerCase().includes('cable input') || d.toLowerCase().includes('cable')) || sourceDevices[0] || '');
    output = outputDevices.includes(savedOutput) ? savedOutput : (outputDevices.find(d => !d.toLowerCase().includes('cable')) || outputDevices[0] || '');
  }

  async function toggleDsp() {
    if (isStarting) return;
    if (isRunning) {
      await invoke('stop_dsp');
      isRunning = false;
      localStorage.setItem('vesper_dsp_is_running', 'false');
    } else {
      await startDsp();
    }
  }

  async function startDsp(restart = false) {
    if (isStarting || (!restart && isRunning)) return;
    isStarting = true;
    try {
      for (let attempt = 0; attempt < 3; attempt += 1) {
        try {
          await invoke('start_dsp', {
            source,
            output,
            isAsio: false, // matches is_asio in rust due to Tauri's camelCase conversion
            headroomDb,
            targetSampleRate: targetRate,
            filterType,
            outputSampleFormat: null
          });
          break;
        } catch (error) {
          const isBusy = String(error).includes('-2147024726') || String(error).includes('리소스가 사용 중');
          if (!isBusy || attempt === 2) throw error;
          await new Promise((resolve) => setTimeout(resolve, 150));
        }
      }
      isRunning = true;
      localStorage.setItem('vesper_dsp_is_running', 'true');
      // 실제 스트림 파라미터를 localStorage에 저장 (시그널 패스 iframe에서 접근용)
      invoke<StreamInfo | null>('get_stream_info').then((info) => {
        if (info) localStorage.setItem('vesper_dsp_stream_info', JSON.stringify(info));
      }).catch(() => {});
    } catch (e) {
      alert("백엔드 오류: " + e);
      isRunning = false;
    } finally {
      isStarting = false;
    }
  }

  async function toggleMute() {
    isMuted = !isMuted;
    await invoke('set_mute', { muted: isMuted });
  }

  async function loadAutoEqIndex() {
    if (autoEqTree.length) return;
    autoEqLoading = true;
    try {
      const response = await fetch('https://api.github.com/repos/opra-project/opra/git/trees/main?recursive=1');
      if (!response.ok) throw new Error(`OPRA request failed`);
      const data = await response.json() as { tree?: Array<{ path?: unknown }> };
      autoEqTree = (data.tree ?? [])
        .filter((entry): entry is { path: string } => typeof entry.path === 'string')
        .filter((entry) => entry.path.startsWith('database/vendors/') && entry.path.endsWith('info.json'));
    } catch (e) {
      console.error(e);
      message = 'AutoEQ 목록을 불러오지 못했습니다.';
    } finally {
      autoEqLoading = false;
    }
  }

  async function searchAutoEq() {
    const query = autoEqQuery.trim().toLowerCase();
    if (!query) {
      autoEqResults = [];
      return;
    }
    
    await loadAutoEqIndex();
    const results: AutoEqResult[] = [];
    
    for (const name of (spinoramaData as string[])) {
      if (name.toLowerCase().includes(query)) {
        results.push({ vendor: 'Spinorama', product: name, mode: 'spinorama' });
      }
    }

    for (const entry of autoEqTree) {
      const parts = entry.path.split('/');
      if (parts.length > 5 && parts[3] === 'products') {
        const vendor = parts[2];
        const product = parts[4];
        if (`${vendor} ${product}`.toLowerCase().includes(query) && !results.some(r => r.vendor === vendor && r.product === product)) {
          results.push({ vendor, product, mode: 'opra' });
        }
      }
    }
    autoEqResults = results.slice(0, 50);
  }

  async function selectAutoEq(result: AutoEqResult) {
    autoEqLoading = true;
    const { vendor, product, mode } = result;
    autoEqMode = mode;
    try {
      let bands: EqBandPayload[] = [];
      let preamp = 0;

      if (autoEqMode === 'spinorama') {
        const response = await fetch(`https://raw.githubusercontent.com/pierreaubert/spinorama/master/datas/eq/${encodeURIComponent(product)}/iir-autoeq.txt`);
        if (!response.ok) throw new Error(`Spinorama request failed`);
        const text = await response.text();
        for (const line of text.split('\n')) {
          if (line.startsWith('Preamp:')) {
            const match = line.match(/Preamp:\s*([-\d.]+)\s*dB/);
            if (match) preamp = parseFloat(match[1]);
          } else if (line.startsWith('Filter')) {
            const match = line.match(/Fc\s+([\d.]+)\s+Hz\s+Gain\s+([-\d.]+)\s+dB\s+Q\s+([\d.]+)/);
            if (match) {
              bands.push({ filter_type: 'peaking', frequency: parseFloat(match[1]), gain_db: parseFloat(match[2]), q: parseFloat(match[3]) });
            }
          }
        }
        autoEqProduct = product;
      } else {
        const profilePath = autoEqTree.find(e => e.path.includes(`vendors/${vendor}/products/${product}/eq/`) && e.path.endsWith('info.json'));
        if (!profilePath) {
          message = 'EQ 프로필을 찾지 못했습니다.';
          return;
        }
        const response = await fetch(`https://raw.githubusercontent.com/opra-project/OPRA/main/${profilePath.path}`);
        if (!response.ok) throw new Error(`AutoEQ request failed`);
        const data = await response.json() as {
          parameters?: { gain_db?: unknown; bands?: Array<Record<string, unknown>> };
        };
        bands = (data.parameters?.bands ?? [])
          .map(normalizeEqBand)
          .filter((band): band is EqBandPayload => band !== null);
        preamp = finiteNumber(data.parameters?.gain_db, 0);
        autoEqProduct = `${vendor} · ${product}`;
      }
      
      autoEqQuery = autoEqProduct;
      autoEqResults = [];
      localStorage.setItem(`vesper_dsp_opra_${output}`, JSON.stringify({ mode: autoEqMode, vendor, product, enabled: autoEqEnabled, preamp_gain: preamp, bands }));
      
      await invoke('apply_output_eq_profile', {
        profile: { enabled: autoEqEnabled, preamp_gain: preamp, bands }
      });
      message = `${autoEqProduct} 적용됨.`;
    } catch (e) {
      console.error(e);
      message = 'AutoEQ 프로필 적용 실패.';
    } finally {
      autoEqLoading = false;
    }
  }

  async function toggleAutoEq() {
    autoEqEnabled = !autoEqEnabled;
    if (autoEqProduct) {
      const savedOpraStr = localStorage.getItem(`vesper_dsp_opra_${output}`);
      const saved = parseSavedAutoEq(savedOpraStr);
      if (!saved) {
        autoEqEnabled = false;
        message = '저장된 AutoEQ 프로필을 다시 선택해주세요.';
        return;
      }
      localStorage.setItem(
        `vesper_dsp_opra_${output}`,
        JSON.stringify({ ...saved, enabled: autoEqEnabled })
      );
      
      if (!autoEqEnabled) {
        await invoke('apply_output_eq_profile', { profile: { enabled: false, preamp_gain: 0, bands: [] } });
        emit('update-signal-path');
      } else {
        if (saved.mode === 'opra') await loadAutoEqIndex();
        await selectAutoEq(saved);
      }
    }
  }

  function finiteNumber(value: unknown, fallback: number): number {
    const parsed = typeof value === 'number' ? value : Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function normalizeEqBand(band: Record<string, unknown>): EqBandPayload | null {
    const frequency = finiteNumber(band.frequency ?? band.fc, Number.NaN);
    const gainDb = finiteNumber(band.gain_db ?? band.gain, Number.NaN);
    const q = finiteNumber(band.q, Number.NaN);
    if (![frequency, gainDb, q].every(Number.isFinite) || frequency <= 0 || q <= 0) return null;
    return {
      filter_type: String(band.filter_type ?? band.type ?? 'peaking').toLowerCase(),
      frequency,
      gain_db: gainDb,
      q
    };
  }

  function parseSavedAutoEq(value: string | null): SavedAutoEq | null {
    if (!value) return null;
    try {
      const parsed = JSON.parse(value) as Partial<SavedAutoEq>;
      if (typeof parsed.vendor !== 'string' || typeof parsed.product !== 'string') return null;
      return {
        vendor: parsed.vendor,
        product: parsed.product,
        mode: parsed.mode === 'spinorama' ? 'spinorama' : 'opra',
        enabled: parsed.enabled === true,
        preamp_gain: parsed.preamp_gain ?? 0,
        bands: parsed.bands ?? []
      };
    } catch {
      return null;
    }
  }

  function normalizeResamplerFilter(value: string): ResamplerFilter {
    switch (value) {
      case 'linear_precise':
      case '정밀한, 선형 위상':
        return 'linear_precise';
      case 'minimum_phase':
      case '정확한 최소 단계':
      case '정확한 최소 단계 (Minimum Phase)':
        return 'minimum_phase';
      case 'linear_smooth':
      case '부드러움, 리니어 위상':
        return 'linear_smooth';
      case 'phase_smooth':
      case '부드러움, 위상 변화 최소화':
        return 'phase_smooth';
      default:
        return 'minimum_phase';
    }
  }

  // Settings Functions
  async function checkAutoStartStatus() {
    try {
      autoStartEnabled = await isEnabled();
    } catch (e) {
      console.error('Failed to check autostart:', e);
    }
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
      console.error('Failed to toggle autostart:', e);
    }
  }

  function loadLockedWindowPosition(): WindowPosition | null {
    const stored = localStorage.getItem(windowLockPositionKey);
    if (!stored) return null;

    try {
      const position = JSON.parse(stored) as WindowPosition;
      return Number.isFinite(position.x) && Number.isFinite(position.y) ? position : null;
    } catch {
      return null;
    }
  }

  async function saveLockedWindowPosition() {
    const position = await getCurrentWindow().outerPosition();
    lockedWindowPosition = { x: position.x, y: position.y };
    localStorage.setItem(windowLockPositionKey, JSON.stringify(lockedWindowPosition));
  }

  async function restoreLockedWindowPosition() {
    if (!isWindowLocked || !lockedWindowPosition || isRestoringLockedPosition) return;

    isRestoringLockedPosition = true;
    try {
      await getCurrentWindow().setPosition(new PhysicalPosition(lockedWindowPosition.x, lockedWindowPosition.y));
    } finally {
      setTimeout(() => { isRestoringLockedPosition = false; }, 0);
    }
  }

  async function toggleWindowLock() {
    if (isWindowLocked) {
      isWindowLocked = false;
      lockedWindowPosition = null;
      localStorage.removeItem('vesper_dsp_window_locked');
      localStorage.removeItem(windowLockPositionKey);
      return;
    }

    try {
      await saveLockedWindowPosition();
      isWindowLocked = true;
      localStorage.setItem('vesper_dsp_window_locked', 'true');
    } catch (e) {
      console.error('Failed to lock window position:', e);
    }
  }

  async function checkForUpdates() {
    if (isCheckingUpdate) return;
    isCheckingUpdate = true;
    updateStatus = '확인 중...';
    try {
      const update = await checkUpdate();
      if (update) {
        hasUpdate = true;
        newVersion = update.version;
        updateBody = update.body || '새로운 기능 및 버그 수정이 포함되어 있습니다.';
        updateStatus = '업데이트 가능';
      } else {
        hasUpdate = false;
        updateStatus = '최신 버전입니다';
        setTimeout(() => { updateStatus = '업데이트 확인'; }, 3000);
      }
    } catch (e) {
      console.error('Update failed:', e);
      updateStatus = '업데이트 실패';
      setTimeout(() => { updateStatus = '업데이트 확인'; }, 3000);
    } finally {
      isCheckingUpdate = false;
    }
  }

  async function installUpdate() {
    if (isCheckingUpdate) return;
    isCheckingUpdate = true;
    updateStatus = '다운로드 및 설치 중...';
    try {
      const update = await checkUpdate();
      if (!update) {
        hasUpdate = false;
        updateStatus = '최신 버전입니다';
        return;
      }
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      console.error('Update install failed:', e);
      updateStatus = '설치 실패. 다시 시도해주세요.';
    } finally {
      isCheckingUpdate = false;
    }
  }

  onMount(async () => {
    await fetchDevices();
    currentVersion = await getVersion();

    if (isWindowLocked) {
      lockedWindowPosition = loadLockedWindowPosition();
      if (lockedWindowPosition) {
        await restoreLockedWindowPosition();
      } else {
        await saveLockedWindowPosition();
      }
    }

    unlistenWindowMoved = await getCurrentWindow().onMoved(({ payload }) => {
      if (!isWindowLocked || !lockedWindowPosition || isRestoringLockedPosition) return;
      if (payload.x !== lockedWindowPosition.x || payload.y !== lockedWindowPosition.y) {
        void restoreLockedWindowPosition();
      }
    });
    
    const t = localStorage.getItem('vesper_dsp_target_rate'); if (t) targetRate = Number(t);
    const st = localStorage.getItem('vesper_dsp_strategy'); if (st) strategy = st;
    const f = localStorage.getItem('vesper_dsp_filter'); if (f) filterType = normalizeResamplerFilter(f);
    const df = localStorage.getItem('vesper_dsp_dsd_filter'); if (df) dsdFilter = df;
    const dg = localStorage.getItem('vesper_dsp_dsd_gain'); if (dg) dsdGain = dg;
    const h = localStorage.getItem('vesper_dsp_headroom'); if (h) headroomDb = Number(h);
    const clipping = localStorage.getItem('vesper_dsp_show_clipping'); if (clipping !== null) showClipping = clipping === 'true';

    settingsRestored = true;

    eventUnlisteners.push(await listen('clipping-detected', () => {
      if (!showClipping) return;
      isClipping = true;
      clearTimeout(clipTimer);
      clipTimer = setTimeout(() => { isClipping = false; }, 500);
    }));

    eventUnlisteners.push(await listen('open-settings', async () => {
      showSettings = true;
      await checkAutoStartStatus();
    }));

    eventUnlisteners.push(await listen<EngineError>('engine-error', ({ payload }) => {
      isRunning = false;
      isStarting = false;
      localStorage.setItem('vesper_dsp_is_running', 'false');
      message = `${payload.stage === 'input' ? '입력' : '출력'} 스트림 오류: ${payload.message}`;
    }));

    const savedIsRunning = localStorage.getItem('vesper_dsp_is_running');
    const engineStatus = await invoke<EngineStatus>('get_engine_status');
    isRunning = engineStatus.running;
    if (!engineStatus.running && savedIsRunning === 'true') {
      setTimeout(() => { startDsp(); }, 500);
    }

    window.addEventListener('message', handleWindowMessage);
  });

  onDestroy(() => {
    unlistenWindowMoved?.();
    eventUnlisteners.forEach((unlisten) => unlisten());
    eventUnlisteners = [];
    window.removeEventListener('message', handleWindowMessage);
    clearTimeout(clipTimer);
    clearTimeout(restartTimer);
  });

  function handleWindowMessage(event: MessageEvent) {
    if (
      typeof event.data === 'object' &&
      event.data !== null &&
      'type' in event.data &&
      event.data.type === 'resize_signal_modal' &&
      'height' in event.data
    ) {
      signalModalHeight = Math.min(finiteNumber(event.data.height, 440) + 60, 600);
    }
  }

  let restartTimer: ReturnType<typeof setTimeout>;
  let savedSettingsSignature = '';
  let engineSettingsSignature = '';

  $: {
    const nextSavedSettingsSignature = JSON.stringify([
      source,
      output,
      targetRate,
      strategy,
      filterType,
      dsdFilter,
      dsdGain,
      headroomDb,
      showClipping
    ]);
    if (settingsRestored && nextSavedSettingsSignature !== savedSettingsSignature) {
      savedSettingsSignature = nextSavedSettingsSignature;
      save();
    }
  }

  $: {
    const nextEngineSettingsSignature = JSON.stringify([
      source,
      output,
      targetRate,
      filterType,
      headroomDb
    ]);
    if (settingsRestored && nextEngineSettingsSignature !== engineSettingsSignature) {
      const shouldRestart = engineSettingsSignature !== '';
      engineSettingsSignature = nextEngineSettingsSignature;
      if (shouldRestart && isRunning && typeof window !== 'undefined') {
        clearTimeout(restartTimer);
        restartTimer = setTimeout(() => {
          startDsp(true);
        }, 500); // 0.5초 디바운스 (슬라이더 조작 시 뚝뚝 끊김 방지)
      }
    }
  }

  $: if (settingsRestored && output) {
    applySavedAutoEqForOutput(output);
  }
</script>

<div class="relative w-screen h-screen transition-all duration-1000 ease-in-out"
     style="box-shadow: 0 0 60px {isRunning ? 'rgba(10, 132, 255, 0.25)' : 'rgba(255,255,255,0.02)'};">
  
  <div class="liquid-glass w-full h-full flex flex-col relative overflow-hidden">
    
    <div class="flex-1 min-w-0 min-h-0 flex flex-col gap-4 p-4 pt-4 overflow-hidden">
      
      <div class="flex items-center justify-between z-10 pb-2 gap-2 {isWindowLocked ? '' : 'cursor-grab active:cursor-grabbing'}" 
           role="presentation"
           on:mousedown={() => { if (!isWindowLocked) getCurrentWindow().startDragging(); }}>
        <div class="pointer-events-none flex items-center gap-1.5 shrink-0">
          <h1 class="text-lg font-bold tracking-tight text-white/90 whitespace-nowrap">Vesper <span class="text-white/30 font-normal mx-0.5">|</span> DSP</h1>
          <div class="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-white/5 border border-white/10 ring-0 shadow-none shrink-0" style="border-color: rgba(255, 255, 255, 0.1);">
            <div class="h-1.5 w-1.5 rounded-full transition-colors duration-300 {isRunning ? 'bg-[#0A84FF] shadow-[0_0_8px_rgba(10,132,255,0.7)]' : 'bg-white/20 shadow-none'}"></div>
            <span class="text-[8px] font-bold text-white/50 tracking-widest uppercase mt-px">{isRunning ? 'Active' : 'Standby'}</span>
          </div>
        </div>
        
        <div class="flex items-center gap-3 pr-0.5 shrink-0" role="presentation" on:mousedown|stopPropagation>
          <div class="flex items-center gap-2">
          <button on:click={async () => { showSettings = true; await checkAutoStartStatus(); }} class="flex items-center justify-center w-5 h-5 bg-transparent transition-colors group" title="환경설정" aria-label="환경설정">
            <svg class="w-3.5 h-3.5 text-white/40 group-hover:text-white/70 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
          </button>
          <button on:click={() => isSignalModalOpen = true} class="flex items-center justify-center w-5 h-5 bg-transparent transition-colors group" title="시그널 패스 보기" aria-label="시그널 패스 보기">
            <svg class="w-3.5 h-3.5 text-white/40 group-hover:text-white/70 transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012-2h-2a2 2 0 01-2-2z"></path></svg>
          </button>
          </div>
          <div class="flex items-center gap-2">
          <button on:click={() => getCurrentWindow().minimize()} class="w-3 h-3 rounded-full bg-white/20 hover:bg-yellow-500 transition-colors" aria-label="최소화"></button>
          <button on:click={() => getCurrentWindow().hide()} class="w-3 h-3 rounded-full bg-white/20 hover:bg-red-500 transition-colors" aria-label="트레이로 숨기기"></button>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-4">
        
        <div class="flex flex-col gap-3.5">
          <div class="w-full flex flex-col gap-1.5 relative group">
            <span class="text-[10px] font-semibold tracking-widest text-white/50 uppercase pl-1">Input Source</span>
            <CustomSelect bind:value={source} options={sourceDevices.map(d => ({ value: d, label: d }))} bind:isOpen={sourceMenuOpen} />
          </div>
          <div class="w-full flex flex-col gap-1.5 relative group">
            <span class="text-[10px] font-semibold tracking-widest text-white/50 uppercase pl-1">Output Device</span>
            <CustomSelect bind:value={output} options={outputDevices.map(d => ({ value: d, label: d }))} bind:isOpen={outputMenuOpen} />
          </div>
        </div>

        <div class="flex flex-col gap-3.5 border-t border-white/5 pt-2.5 relative" class:z-50={strategyMenuOpen || filterMenuOpen || dsdFilterMenuOpen || dsdGainMenuOpen}>
          <h3 class="text-xs font-bold tracking-widest text-white/70 uppercase flex items-center gap-2">
            <svg class="w-4 h-4 text-apple-blue" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012-2h-2a2 2 0 01-2-2z"></path></svg>
            DSP 엔진 설정
          </h3>
          
          <div class="flex flex-col gap-3">
            <div class="flex justify-between items-center gap-4">
              <div class="flex-1">
                <p class="text-[11px] font-semibold text-white/90">샘플 레이트 변환</p>
                <p class="text-[9px] text-white/40 mt-0.5">출력 샘플 속도 관리 방법</p>
              </div>
              <div class="w-44 shrink-0 text-right relative" class:z-50={strategyMenuOpen}>
                <CustomSelect bind:value={strategy} bind:isOpen={strategyMenuOpen} options={strategyOptions} align="right" />
              </div>
            </div>

            <div class="flex justify-between items-center gap-4">
              <div class="flex-1">
                <p class="text-[11px] font-semibold text-white/90">리샘플링 필터</p>
              </div>
              <div class="w-44 shrink-0 text-right relative" class:z-50={filterMenuOpen}>
                <CustomSelect bind:value={filterType} bind:isOpen={filterMenuOpen} options={filterOptions} align="right" />
              </div>
            </div>
            
            <div class="flex justify-between items-center gap-4 border-t border-white/5 pt-3 mt-1">
              <div class="flex-1">
                <p class="text-[11px] font-semibold text-white/90">DSD ▶ PCM 필터</p>
              </div>
              <div class="w-44 shrink-0 text-right relative" class:z-50={dsdFilterMenuOpen}>
                <CustomSelect bind:value={dsdFilter} bind:isOpen={dsdFilterMenuOpen} options={dsdFilterOptions} align="right" />
              </div>
            </div>

            <div class="flex justify-between items-center gap-4">
              <div class="flex-1">
                <p class="text-[11px] font-semibold text-white/90">DSD ▶ PCM 게인</p>
              </div>
              <div class="w-32 shrink-0 text-right relative" class:z-50={dsdGainMenuOpen}>
                <CustomSelect bind:value={dsdGain} bind:isOpen={dsdGainMenuOpen} options={dsdGainOptions} align="right" />
              </div>
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-3 border-t border-white/5 pt-3 relative" class:z-50={autoEqResults.length > 0}>
          
          <div class="flex flex-col gap-2 relative">
            <div class="flex justify-between items-center mb-1">
              <div class="flex items-center gap-2">
                <span class="text-[11px] font-bold tracking-widest text-apple-blue uppercase">자동 EQ</span>
                <span class="text-[9px] text-white/50 truncate max-w-[150px]" title={autoEqProduct}>
                  {autoEqProduct ? `[${autoEqMode === 'spinorama' ? '스피커' : '이어폰'}] ${autoEqProduct}` : '미설정'}
                </span>
              </div>
              <button on:click={toggleAutoEq} aria-label="자동 EQ 전환" class="flex items-center justify-center w-5 h-5 rounded-full transition-colors group {autoEqEnabled ? 'bg-apple-blue/20 hover:bg-apple-blue/40' : 'bg-white/5 hover:bg-white/20'}">
                <svg class="w-3 h-3 transition-colors {autoEqEnabled ? 'text-apple-blue' : 'text-white/40'}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path></svg>
              </button>
            </div>
            
            <div class="flex gap-2 bg-black/40 p-2 rounded-xl shadow-inner border border-white/5 relative z-10">
              <input class="flex-1 min-w-0 bg-transparent px-2 py-1 text-[12px] text-white outline-none placeholder:text-white/30" bind:value={autoEqQuery} on:keydown={e => e.key==='Enter' && searchAutoEq()} placeholder="영문 모델명 통합 검색 (스피커/이어폰)" />
              <button class="px-4 py-1.5 rounded-lg bg-white/10 text-[11px] font-bold hover:bg-white/20 transition-colors" on:click={searchAutoEq}>
                {autoEqLoading ? '...' : '검색'}
              </button>
            </div>
            
            {#if autoEqResults.length}
              <div class="absolute bottom-full left-0 w-full mb-2 h-64 rounded-xl border border-white/20 bg-[#1e1e20]/95 backdrop-blur-2xl shadow-[0_-10px_40px_rgba(0,0,0,0.5)] z-50 flex flex-col overflow-hidden">
                <div class="flex justify-between items-center px-3 py-2 border-b border-white/10 bg-black/40 shrink-0">
                  <span class="text-[9px] font-bold text-white/50 tracking-widest uppercase">검색 결과</span>
                  <button on:click={() => autoEqResults = []} class="text-[10px] font-bold text-white/40 hover:text-white transition-colors">닫기</button>
                </div>
                <div class="flex-1 overflow-y-auto p-1.5 flex flex-col gap-1">
                  {#each autoEqResults as r}
                    <button class="flex flex-col w-full px-3 py-2.5 text-left rounded-lg hover:bg-white/10 transition-colors gap-0.5" on:click={() => selectAutoEq(r)}>
                      <div class="flex justify-between items-center w-full gap-2">
                        <span class="font-bold text-[11.5px] text-white/90 truncate flex-1">{r.product}</span>
                        <span class="text-[8px] font-bold px-1.5 py-0.5 rounded bg-white/10 text-white/60 shrink-0">{r.mode === 'spinorama' ? '스피커' : '이어폰'}</span>
                      </div>
                      <span class="text-[9px] text-white/40 tracking-wide truncate w-full">{r.vendor}</span>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        </div>
        
        <div class="flex flex-col gap-2 border-t border-white/5 pt-3 transition-all duration-300 {isClipping ? 'bg-red-500/10 ring-1 ring-red-500/50 shadow-[0_0_20px_rgba(239,68,68,0.2)] rounded-xl p-2' : ''}">
          <div class="flex justify-between items-end">
            <span class="text-[10px] font-semibold tracking-wider text-white/50 uppercase">Headroom</span>
            <span class="text-base font-bold tracking-tighter {isClipping ? 'text-red-400' : 'text-white/90'}">{headroomDb.toFixed(1)}<span class="text-[10px] text-white/40 ml-1 font-medium">dB</span></span>
          </div>
          <input type="range" min="-12" max="0" step="0.5" bind:value={headroomDb} class="apple-slider" />
          <div class="flex justify-between items-center text-[9px] text-white/40">
            <label class="flex items-center gap-1.5 cursor-pointer hover:text-white/60 transition-colors">
              <input type="checkbox" bind:checked={showClipping} class="accent-white scale-90" /> 클리핑 감지
            </label>
            {#if isClipping}<span class="font-bold text-red-500 tracking-wider">CLIPPING</span>{/if}
          </div>
        </div>

      </div>
    </div>

    <div class="absolute inset-x-0 bottom-0 z-50 px-4 pb-3 pt-1 bg-[#1C1C1E]">
      <div class="grid grid-cols-[1fr_auto] gap-2.5">
        <button 
          class="w-full py-2.5 rounded-2xl font-bold text-sm tracking-wide uppercase transition-all duration-300 active:scale-[0.97] shadow-lg
                 {isRunning 
                   ? 'bg-apple-blue/20 text-apple-blue border border-apple-blue/30 hover:bg-apple-blue/30 shadow-[0_0_20px_rgba(10,132,255,0.15)]' 
                   : 'bg-white text-black hover:bg-gray-200'}"
          on:click={toggleDsp}
          disabled={isStarting}
        >
          {isRunning ? 'Stop Engine' : 'Engage DSP'}
        </button>
        <button 
          class="flex items-center justify-center px-4 rounded-2xl border transition-all duration-300 active:scale-[0.97]
                 {isMuted ? 'bg-red-500/20 text-red-400 border-red-500/30' : 'bg-black/40 text-white/70 border-white/10 hover:bg-black/60 hover:text-white'}"
          on:click={toggleMute}
        >
          <span class="text-xs font-bold tracking-widest uppercase">{isMuted ? 'Muted' : 'Mute'}</span>
        </button>
      </div>
    </div>
  </div>
</div>

{#if isSignalModalOpen}
  <div class="absolute inset-0 z-50 flex items-center justify-center p-6 bg-black/60 backdrop-blur-md animate-in fade-in duration-200" role="presentation" on:mousedown|self={() => isSignalModalOpen = false}>
    <div class="w-full max-w-sm bg-white/10 backdrop-blur-xl border border-white/20 rounded-3xl shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200" style="height: {signalModalHeight}px; max-height: 90vh; transition: height 0.3s cubic-bezier(0.4, 0, 0.2, 1);">
      <div class="flex items-center justify-between p-4 border-b border-white/10 bg-black/20">
        <h2 class="text-lg font-bold text-white tracking-wide">Signal Path</h2>
        <button on:click={() => isSignalModalOpen = false} aria-label="시그널 패스 닫기" class="w-8 h-8 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white/70 hover:text-white transition-colors cursor-pointer">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>
      <div class="flex-1 w-full bg-transparent">
        <iframe 
          src="/signal?is_modal=true&eq={eqEnabled}&eq_name={encodeURIComponent(autoEqProduct)}" 
          class="w-full h-full border-none bg-transparent"
          title="Signal Path Viewer"
        ></iframe>
      </div>
    </div>
  </div>
{/if}

{#if showSettings}
  <div class="absolute inset-0 z-50 flex items-center justify-center p-5 bg-black/60 backdrop-blur-md animate-in fade-in duration-200" role="presentation" on:click|self={() => showSettings = false}>
    <div class="w-full max-w-sm bg-[#0E0E10]/95 border border-white/10 rounded-2xl flex flex-col shadow-[0_8px_32px_rgba(0,0,0,0.8)] overflow-hidden">
      <div class="flex items-center justify-between p-4 border-b border-white/5 bg-white/5">
        <h2 class="text-sm font-bold tracking-tight text-white/90 [&>svg]:hidden">
          <svg class="w-5 h-5 text-apple-blue" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
          환경설정
        </h2>
        <button on:click={() => showSettings = false} class="w-6 h-6 rounded-full bg-white/10 flex items-center justify-center hover:bg-white/20 transition-colors" aria-label="환경설정 닫기">
          <svg class="w-3 h-3 text-white/70" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>

      <div class="p-5 flex-1 space-y-6">
        <div class="space-y-3">
        <h3 class="text-[10px] font-bold tracking-widest text-white/50 uppercase">System</h3>
        <div class="flex justify-between items-center bg-black/30 p-4 rounded-xl border border-white/5">
          <div>
            <p class="text-xs font-semibold text-white/90">Windows 시작 시 자동 실행</p>
            <p class="text-[9px] text-white/50 mt-1">부팅 시 백그라운드로 자동 실행</p>
          </div>
          <button
            on:click={toggleAutoStart}
            aria-label="Windows 시작 시 자동 실행 전환"
            class="w-10 h-5 rounded-full transition-colors {autoStartEnabled ? 'bg-green-500' : 'bg-white/20'} relative"
          >
            <div class="absolute w-4 h-4 bg-white rounded-full top-[2px] transition-transform {autoStartEnabled ? 'translate-x-5' : 'translate-x-[2px]'} shadow-sm"></div>
          </button>
        </div>

        <div class="flex justify-between items-center bg-black/30 p-4 rounded-xl border border-white/5 mt-2">
          <div>
            <p class="text-xs font-semibold text-white/90">창 위치 잠금 (이동 방지)</p>
            <p class="text-[9px] text-white/50 mt-1">원하는 곳에 둔 후 켜두면 항상 그 위치에 고정됨</p>
          </div>
          <button 
            on:click={toggleWindowLock}
            aria-label="창 위치 잠금 전환"
            class="w-10 h-5 rounded-full transition-colors {isWindowLocked ? 'bg-apple-blue' : 'bg-white/20'} relative"
          >
            <div class="absolute w-4 h-4 bg-white rounded-full top-[2px] transition-transform {isWindowLocked ? 'translate-x-5' : 'translate-x-[2px]'} shadow-sm"></div>
          </button>
        </div>
        </div>

        <div class="space-y-3">
          <h3 class="text-[10px] font-bold tracking-widest text-white/50 uppercase">Updates</h3>
          <div class="flex flex-col bg-black/30 p-4 rounded-xl border border-white/5 gap-3">
            <div class="flex justify-between items-center">
              <div>
                <p class="text-xs font-semibold text-white/90">현재 버전</p>
                <p class="text-[10px] text-white/50 font-mono mt-0.5">v{currentVersion || '…'}</p>
              </div>
              {#if !hasUpdate}
                <button on:click={checkForUpdates} disabled={isCheckingUpdate} class="px-3 py-1.5 text-[10px] font-semibold rounded-lg bg-apple-blue/10 text-apple-blue hover:bg-apple-blue/20 transition-colors flex items-center gap-1.5 {isCheckingUpdate ? 'opacity-50 cursor-not-allowed' : ''}">
                  {updateStatus}
                </button>
              {/if}
            </div>

            {#if hasUpdate}
              <div class="border-t border-white/5 pt-3 mt-1">
                <p class="text-xs font-bold text-green-400 mb-1">새 버전 발견: v{newVersion}</p>
                <p class="text-[10px] text-white/60 mb-3 leading-relaxed break-keep">{updateBody}</p>
                <button on:click={installUpdate} disabled={isCheckingUpdate} class="w-full py-2 text-[11px] font-bold rounded-lg bg-green-500/20 text-green-400 hover:bg-green-500/30 transition-colors {isCheckingUpdate ? 'opacity-50 cursor-not-allowed' : ''}">
                  {isCheckingUpdate ? updateStatus : '지금 다운로드 및 다시 시작'}
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
            <p class="text-[9px] text-white/50 mt-1">Vesper DSP에 사용된 오픈소스 라이선스를 확인합니다.</p>
          </div>
          <button 
            on:click={() => showLicense = true}
            title="고지 보기"
            class="w-8 h-8 rounded-full flex items-center justify-center bg-white/5 border border-white/10 hover:bg-white/10 hover:border-white/20 transition-colors group"
          >
            <svg class="w-4 h-4 text-white/50 group-hover:text-white transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
          </button>
        </div>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if showLicense}
  <div class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-black/80 backdrop-blur-md transition-opacity" role="presentation" on:click|self={() => showLicense = false}>
    <div class="relative w-full max-w-md bg-[#0E0E10] border border-white/10 rounded-3xl p-6 shadow-2xl flex flex-col gap-6">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold tracking-tight text-white/90">오픈소스 고지</h2>
        <button on:click={() => showLicense = false} aria-label="오픈소스 고지 닫기" class="w-8 h-8 rounded-full bg-white/5 hover:bg-white/10 flex items-center justify-center transition-colors">
          <svg class="w-4 h-4 text-white/50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>
      
      <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto pr-2 custom-scrollbar">
        <div class="p-4 rounded-xl bg-white/5 border border-white/5">
          <h3 class="text-sm font-bold text-white/90 mb-2">AutoEq / OPRA</h3>
          <p class="text-xs text-white/60 mb-2 leading-relaxed">
            이어폰/헤드폰용 이퀄라이제이션(EQ) 프로파일 기능은 AutoEq 프로젝트 및 OPRA 데이터베이스를 참조합니다.
          </p>
          <div class="text-[10px] text-white/40 font-mono bg-black/40 p-3 rounded-lg leading-relaxed">
            MIT License<br><br>
            Copyright (c) 2018 Jaakko Pasanen (AutoEq)<br>
            Copyright (c) OPRA Project<br><br>
            Permission is hereby granted, free of charge, to any person obtaining a copy
            of this software and associated documentation files (the "Software")...
          </div>
        </div>
        
        <div class="p-4 rounded-xl bg-white/5 border border-white/5">
          <h3 class="text-sm font-bold text-white/90 mb-2">Spinorama</h3>
          <p class="text-xs text-white/60 mb-2 leading-relaxed">
            스피커용 룸 튜닝 EQ 프로파일 기능은 오픈소스 스피커 측정 데이터베이스인 Spinorama.org 를 기반으로 제공됩니다.
          </p>
          <div class="text-[10px] text-white/40 font-mono bg-black/40 p-3 rounded-lg leading-relaxed">
            Copyright (c) Pierre Aubert<br><br>
            Data sources include Audio Science Review, ErinsAudioCorner, and others.
          </div>
        </div>

        <button 
          on:click={() => showThirdPartyLicense = !showThirdPartyLicense}
          class="w-full p-4 rounded-xl bg-white/5 hover:bg-white/10 border border-white/5 transition-colors flex items-center justify-between group"
        >
          <h3 class="text-sm font-bold text-white/90">제3자 오픈소스 라이선스 보기</h3>
          <svg class="w-4 h-4 text-white/50 group-hover:text-white transition-colors transform {showThirdPartyLicense ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
        </button>

        {#if showThirdPartyLicense}
        <div class="p-4 rounded-xl bg-black/40 border border-white/5 text-[10px] text-white/40 font-mono leading-relaxed whitespace-pre-wrap">
Vesper DSP incorporates the following third-party open source libraries and frameworks:

- Tauri (MIT / Apache-2.0)
- Svelte & SvelteKit (MIT)
- Vite (MIT)
- cpal (Apache-2.0)
- rubato (MIT)
- biquad (MIT / Apache-2.0)
- ringbuf (MIT / Apache-2.0)
- tokio (MIT)
- serde (MIT / Apache-2.0)

For libraries licensed under MIT License:
Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
        </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .apple-slider {
    -webkit-appearance: none;
    width: 100%;
    height: 6px;
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
    outline: none;
    transition: background 0.3s;
  }
  .apple-slider:hover {
    background: rgba(255, 255, 255, 0.25);
  }
  .apple-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0,0,0,0.4);
    transition: transform 0.1s;
  }
  .apple-slider::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }
</style>
