<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { page } from '$app/stores';
  import CustomSelect from '$lib/components/CustomSelect.svelte';

  // 고급 DSP 설정 상태 변수 (Roon Style)
  let headroomDb = -3.0;
  let showClipping = false;
  let sampleRateStrategy = '호환성 위주';
  let sampleRateFilter = '정확한 최소 단계';
  let dsdFilter = '권장함 (30kHz Low Pass Filter)';
  let dsdGain = '+6.0dB';
  
  let supportedSampleRates: number[] = [];
  let targetSampleRate: number | null = null;
  
  let targetType = 'earphone';
  let targetDeviceName = '';
  let lsKey = '';
  let isModal = false;

  // OPRA 연동 상태
  // OPRA 연동 상태
  let opraEnabled = false;
  let opraSearchQuery = '';
  let opraSearchResults: { vendor: string, product: string, path: string }[] = [];
  let isOpraDropdownOpen = false;
  let selectedVendor = '';
  let selectedProduct = '';
  let opraLoading = false;
  let opraProfilesTree: any[] = [];
  let eqProfile: any = null;

  async function fetchOpraIndex() {
    if (opraProfilesTree.length > 0) return;
    opraLoading = true;
    try {
      const res = await fetch('https://api.github.com/repos/opra-project/opra/git/trees/main?recursive=1');
      const data = await res.json();
      opraProfilesTree = data.tree.filter((t: any) => t.path.startsWith('database/vendors/') && t.path.endsWith('info.json'));
    } catch (e) {
      console.error(e);
    }
    opraLoading = false;
  }

  function onOpraSearch() {
    if (!opraSearchQuery.trim()) {
      opraSearchResults = [];
      isOpraDropdownOpen = false;
      return;
    }
    const query = opraSearchQuery.toLowerCase();
    const results: { vendor: string, product: string, path: string }[] = [];
    
    for (const t of opraProfilesTree) {
      const parts = t.path.split('/');
      if (parts.length > 5 && parts[3] === 'products') {
        const vendor = parts[2];
        const product = parts[4];
        const searchStr = `${vendor} ${product}`.toLowerCase();
        
        if (searchStr.includes(query)) {
          if (!results.some(r => r.vendor === vendor && r.product === product)) {
            results.push({ vendor, product, path: t.path });
          }
        }
      }
    }
    opraSearchResults = results.slice(0, 50); // 성능을 위해 최대 50개 제한
    isOpraDropdownOpen = opraSearchResults.length > 0;
  }

  async function selectOpraProduct(vendor: string, product: string) {
    selectedVendor = vendor;
    selectedProduct = product;
    opraSearchQuery = `${vendor} - ${product}`;
    isOpraDropdownOpen = false;
    
    const profilePath = opraProfilesTree.find(t => 
      t.path.includes(`vendors/${vendor}/products/${product}/eq/`) && t.path.endsWith('info.json')
    );
    if (profilePath) {
      opraLoading = true;
      try {
        const res = await fetch(`https://raw.githubusercontent.com/opra-project/OPRA/main/${profilePath.path}`);
        const data = await res.json();
        if (data.type === 'parametric_eq') {
           eqProfile = {
             enabled: opraEnabled,
             preamp_gain: data.parameters.gain_db || 0.0,
             bands: data.parameters.bands || []
           };
           applyEqProfile(opraEnabled);
        }
      } catch (e) {
        console.error(e);
      }
      opraLoading = false;
    } else {
      eqProfile = null;
      applyEqProfile(false);
    }
  }

  function applyEqProfile(enabled: boolean) {
    if (eqProfile) {
      eqProfile.enabled = enabled;
      invoke('apply_earphone_eq_profile', { profile: eqProfile }).catch(console.error);
      if (targetDeviceName) {
        localStorage.setItem(`ws_opra_vendor_${targetDeviceName}`, selectedVendor);
        localStorage.setItem(`ws_opra_product_${targetDeviceName}`, selectedProduct);
        localStorage.setItem(`ws_opra_enabled_${targetDeviceName}`, enabled.toString());
      }
    } else {
      invoke('apply_earphone_eq_profile', { profile: { enabled: false, preamp_gain: 0.0, bands: [] } }).catch(console.error);
    }
  }

  function loadProfile(deviceName: string) {
    if (!deviceName || typeof window === 'undefined') return;
    const h = localStorage.getItem(`ws_headroom_${deviceName}`); if (h) headroomDb = Number(h);
    const c = localStorage.getItem(`ws_clipping_${deviceName}`); if (c) showClipping = c === 'true';
    const ss = localStorage.getItem(`ws_srStrategy_${deviceName}`); if (ss) sampleRateStrategy = ss;
    const sf = localStorage.getItem(`ws_srFilter_${deviceName}`); if (sf) sampleRateFilter = sf;
    const tsr = localStorage.getItem(`ws_targetSr_${deviceName}`); 
    targetSampleRate = tsr ? Number(tsr) : null;
    const df = localStorage.getItem(`ws_dsdFilter_${deviceName}`); if (df) dsdFilter = df;
    const dg = localStorage.getItem(`ws_dsdGain_${deviceName}`); if (dg) dsdGain = dg;

    // 백엔드에서 기기가 지원하는 실제 샘플 레이트 목록 조회
    let apiDeviceName = deviceName.includes('FiiO') ? deviceName : deviceName.split('(')[0].trim();
    invoke<number[]>('get_device_supported_sample_rates', { deviceName: apiDeviceName })
      .then((rates) => {
        supportedSampleRates = rates;
        if (!targetSampleRate && rates.length > 0) {
          targetSampleRate = rates[rates.length - 1]; // 기본적으로 최대 주파수 선택
        }
      })
      .catch((err) => {
        console.error("Failed to fetch sample rates:", err);
      });

    // OPRA 복구
    const oV = localStorage.getItem(`ws_opra_vendor_${deviceName}`);
    const oP = localStorage.getItem(`ws_opra_product_${deviceName}`);
    const oE = localStorage.getItem(`ws_opra_enabled_${deviceName}`);
    if (oV && oP) {
      opraEnabled = oE === 'true';
      selectedVendor = oV;
      selectedProduct = oP;
      opraSearchQuery = `${oV} - ${oP}`;
      fetchOpraIndex().then(() => {
        selectOpraProduct(oV, oP);
      });
    } else {
      opraEnabled = false;
      opraSearchQuery = '';
      selectedVendor = '';
      selectedProduct = '';
      eqProfile = null;
      applyEqProfile(false);
    }
  }

  onMount(() => {
    // URL에서 target(이어폰/우퍼) 파악
    targetType = $page.url.searchParams.get('target') || 'earphone';
    isModal = $page.url.searchParams.get('is_modal') === 'true';
    lsKey = targetType === 'earphone' ? 'ws_current_earphone' : 'ws_current_speaker';

    // 앱 켤 때 저장된 현재 기기 확인 및 복구
    targetDeviceName = localStorage.getItem(lsKey) || '';
    if (targetDeviceName) {
      loadProfile(targetDeviceName);
    }

    // 다른 창(메인 창)에서 값이 바뀌었을 때도 즉시 반응하도록
    window.addEventListener('storage', (e) => {
      // 메인 창에서 기기를 바꿨을 때 프로필 실시간 교체
      if (e.key === lsKey && e.newValue) {
        targetDeviceName = e.newValue;
        loadProfile(targetDeviceName);
      }
      
      // 현재 기기의 세팅값이 변경되었을 때 UI 동기화
      if (targetDeviceName) {
        if (e.key === `ws_headroom_${targetDeviceName}`) headroomDb = Number(e.newValue);
        if (e.key === `ws_clipping_${targetDeviceName}`) showClipping = e.newValue === 'true';
        if (e.key === `ws_srStrategy_${targetDeviceName}`) sampleRateStrategy = e.newValue || '호환성 위주';
        if (e.key === `ws_srFilter_${targetDeviceName}`) sampleRateFilter = e.newValue || '정확한 최소 단계';
        if (e.key === `ws_targetSr_${targetDeviceName}`) targetSampleRate = e.newValue ? Number(e.newValue) : null;
        if (e.key === `ws_dsdFilter_${targetDeviceName}`) dsdFilter = e.newValue || '권장함 (30kHz Low Pass Filter)';
        if (e.key === `ws_dsdGain_${targetDeviceName}`) dsdGain = e.newValue || '+6.0dB';
      }
    });
  });

  // 상태가 바뀔 때마다 자동으로 백그라운드 저장 (메인 창에 Storage Event 발생)
  $: {
    if (typeof window !== 'undefined' && targetDeviceName) {
      localStorage.setItem(`ws_headroom_${targetDeviceName}`, headroomDb.toString());
      localStorage.setItem(`ws_clipping_${targetDeviceName}`, showClipping.toString());
      localStorage.setItem(`ws_srStrategy_${targetDeviceName}`, sampleRateStrategy);
      localStorage.setItem(`ws_srFilter_${targetDeviceName}`, sampleRateFilter);
      if (targetSampleRate) localStorage.setItem(`ws_targetSr_${targetDeviceName}`, targetSampleRate.toString());
      localStorage.setItem(`ws_dsdFilter_${targetDeviceName}`, dsdFilter);
      localStorage.setItem(`ws_dsdGain_${targetDeviceName}`, dsdGain);
    }
  }
</script>

<div class="relative w-full h-screen bg-[#0E0E10] text-white overflow-y-auto" style="box-shadow: inset 0 0 100px rgba(0,0,0,0.5);">
  <div class="liquid-glass w-full min-h-full flex flex-col relative">
    
    <!-- Header (Drag Region) -->
    {#if !isModal}
    <div 
      class="flex items-center justify-between p-5 pb-3 border-b border-white/10 z-10 sticky top-0 bg-[#0E0E10]/80 backdrop-blur-md cursor-grab active:cursor-grabbing"
      on:mousedown={() => getCurrentWindow().startDragging()}
    >
      <div>
        <h1 class="text-lg font-bold tracking-tight text-white/90">고급 설정</h1>
        <p class="text-xs text-white/50">{targetDeviceName || '연결된 기기 없음'}</p>
      </div>
      <button 
        on:click={() => getCurrentWindow().close()} 
        on:mousedown|stopPropagation
        class="w-5 h-5 rounded-full bg-white/10 hover:bg-red-500 transition-colors flex items-center justify-center"
      >
        <svg class="w-3 h-3 text-white/50 opacity-100" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M6 18L18 6M6 6l12 12"></path></svg>
      </button>
    </div>
    {/if}

    <!-- Content -->
    <div class="flex flex-col gap-8 p-6">

      {#if targetType === 'earphone'}
      <!-- OPRA AutoEQ -->
      <div class="flex flex-col gap-4 relative group">
        <div class="flex justify-between items-center">
          <label class="text-xs font-bold tracking-widest text-white/70 uppercase">AutoEQ (OPRA)</label>
          <button class="w-10 h-5 rounded-full transition-colors {opraEnabled ? 'bg-apple-blue' : 'bg-white/20'} relative" 
            on:click={() => { opraEnabled = !opraEnabled; if(opraProfilesTree.length===0) fetchOpraIndex(); else applyEqProfile(opraEnabled); }}>
            <div class="absolute w-4 h-4 bg-white rounded-full top-[2px] transition-transform {opraEnabled ? 'translate-x-5' : 'translate-x-[2px]'} shadow-sm"></div>
          </button>
        </div>
        
        {#if opraEnabled}
          <div class="flex flex-col gap-3 mt-1 animate-in fade-in duration-200">
            <span class="text-[11px] text-white/50">이어폰/헤드폰 모델 검색 {opraLoading ? '(로딩 중...)' : ''}</span>
            <div class="relative">
              <input type="text" 
                     bind:value={opraSearchQuery} 
                     on:input={onOpraSearch} 
                     on:focus={() => { if(opraSearchQuery) isOpraDropdownOpen = true; }}
                     placeholder="예: simgot em6l" 
                     class="w-full bg-[#1C1C1E] text-sm text-white/90 outline-none border border-white/10 rounded-xl px-4 py-3 focus:border-apple-blue/50 transition-colors" />
              
              {#if isOpraDropdownOpen}
                <div class="absolute top-full left-0 mt-2 w-full max-h-60 overflow-y-auto bg-[#2C2C2E] border border-white/10 rounded-xl shadow-2xl z-50 flex flex-col p-1">
                  {#each opraSearchResults as res}
                    <button class="text-left px-4 py-3 text-sm text-white/90 hover:bg-apple-blue/20 hover:text-apple-blue rounded-lg transition-colors flex flex-col"
                            on:click={() => selectOpraProduct(res.vendor, res.product)}>
                      <span class="font-bold">{res.product}</span>
                      <span class="text-[10px] text-white/40">{res.vendor}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
            {#if eqProfile}
              <p class="text-[11px] text-green-400/80 mt-1">✓ {eqProfile.bands.length}-Band PEQ 프로필이 적용되었습니다.</p>
            {/if}
          </div>
        {/if}
      </div>
      {/if}
      
      <!-- Headroom Management -->
      <div class="flex flex-col gap-3 relative group">
        <div class="flex justify-between items-center">
          <label class="text-xs font-bold tracking-widest text-white/70 uppercase">Headroom Management</label>
          <span class="text-sm font-bold text-white/90">{headroomDb.toFixed(1)} dB</span>
        </div>
        <p class="text-[11px] text-white/40 leading-relaxed mb-1">
          DSP 처리(EQ, 업샘플링 등)로 인해 발생하는 디지털 클리핑을 방지하기 위해 전체 볼륨의 여유 공간(헤드룸)을 미리 확보합니다. -3dB를 권장합니다.
        </p>
        <input type="range" min="-10" max="0" step="0.5" bind:value={headroomDb} class="apple-slider w-full" />
        <div class="flex justify-between items-center mt-3 p-3 bg-black/20 rounded-xl border border-white/5">
          <span class="text-xs text-white/60 font-medium">클리핑 상태 보기 (Clipping Indicator)</span>
          <button class="w-10 h-5 rounded-full transition-colors {showClipping ? 'bg-green-500' : 'bg-white/20'} relative" on:click={() => showClipping = !showClipping}>
            <div class="absolute w-4 h-4 bg-white rounded-full top-[2px] transition-transform {showClipping ? 'translate-x-5' : 'translate-x-[2px]'} shadow-sm"></div>
          </button>
        </div>
      </div>

      <!-- Sample Rate Conversion -->
      <div class="flex flex-col gap-4 relative group border-t border-white/5 pt-6">
        <label class="text-xs font-bold tracking-widest text-white/70 uppercase">Sample Rate Conversion</label>
        
        <div class="flex flex-col gap-1.5">
          <span class="text-[11px] text-white/50">변환 전략</span>
          <div class="relative">
            <CustomSelect 
              bind:value={sampleRateStrategy} 
              options={[
                { value: '호환성 위주', label: '호환성 위주' },
                { value: '최대 PCM 비율', label: '최대 PCM 비율' },
                { value: '최대 PCM 레이트 (2의 제곱)', label: '최대 PCM 레이트 (2의 제곱)' },
                { value: '개별설정', label: '개별설정' }
              ]} 
            />
          </div>
        </div>

        {#if sampleRateStrategy === '개별설정'}
          <div class="flex flex-col gap-1.5 mt-2 transition-all">
            <span class="text-[11px] text-apple-blue font-bold tracking-widest">타겟 주파수 강제 설정 (Upsampling/Downsampling)</span>
            <div class="relative">
              <CustomSelect 
                bind:value={targetSampleRate}
                textClass="text-apple-blue font-bold"
                options={supportedSampleRates.length > 0 
                  ? supportedSampleRates.map(sr => ({ value: sr, label: `${sr} Hz` }))
                  : [{ value: null, label: '지원되는 주파수를 불러올 수 없습니다' }]
                }
              />
            </div>
          </div>
        {/if}

        <div class="flex flex-col gap-1.5 mt-2">
          <span class="text-[11px] text-white/50">변환 필터 특성</span>
          <div class="relative">
            <CustomSelect 
              bind:value={sampleRateFilter} 
              options={[
                { value: '정밀한, 선형 위상', label: '정밀한, 선형 위상' },
                { value: '정확한 최소 단계', label: '정확한 최소 단계 (Minimum Phase)' },
                { value: '부드러운, 리니어 위상', label: '부드러운, 리니어 위상' },
                { value: '부드러움, 위상 변화 최소화', label: '부드러움, 위상 변화 최소화' }
              ]} 
            />
          </div>
        </div>
      </div>

      <!-- DSD Processing -->
      <div class="flex flex-col gap-4 relative group border-t border-white/5 pt-6 pb-4">
        <label class="text-xs font-bold tracking-widest text-white/70 uppercase">DSD Processing</label>
        
        <div class="flex justify-between items-center bg-black/20 p-3 px-4 rounded-xl border border-white/5 relative">
          <span class="text-sm font-medium text-white/80">DSD ▶ PCM Filter</span>
          <div class="w-[220px]">
            <CustomSelect 
              bind:value={dsdFilter} 
              align="right"
              bgClass="bg-transparent border-none shadow-none"
              textClass="text-apple-blue font-bold text-right"
              options={[
                { value: '보편적인 (24kHz Low Pass Filter)', label: '보편적인 (24kHz LPF)' },
                { value: '권장함 (30kHz Low Pass Filter)', label: '권장함 (30kHz LPF)' },
                { value: '허용적인 (50kHz Low pass filter)', label: '허용적인 (50kHz LPF)' },
                { value: '필터링되지 않음 (주의해서 사용)', label: '필터링 안 함' }
              ]}
            />
          </div>
        </div>

        <div class="flex justify-between items-center bg-black/20 p-3 px-4 rounded-xl border border-white/5 relative">
          <span class="text-sm font-medium text-white/80">DSD to PCM Gain</span>
          <div class="w-[120px]">
            <CustomSelect 
              bind:value={dsdGain} 
              align="right"
              bgClass="bg-transparent border-none shadow-none"
              textClass="text-white/90 font-bold text-right"
              options={[
                { value: '+6.0dB', label: '+6.0dB' },
                { value: '+5.0dB', label: '+5.0dB' },
                { value: '+4.0dB', label: '+4.0dB' },
                { value: '+3.0dB', label: '+3.0dB' },
                { value: '+2.0dB', label: '+2.0dB' },
                { value: '+1.0dB', label: '+1.0dB' },
                { value: '+0.0dB', label: '+0.0dB' }
              ]}
            />
          </div>
        </div>
      </div>

    </div>
  </div>
</div>
