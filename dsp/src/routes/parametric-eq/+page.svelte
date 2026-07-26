<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  
  interface EqBand {
    frequency: number;
    gain_db: number;
    q: number;
    filter_type: string;
  }

  // 7가지 필터 타입 정의 (아이콘 및 텍스트)
  const filterTypes = [
    { value: 'peaking', label: 'Peaking', icon: 'M3 12h4l3 -9l5 18l3 -9h4' },
    { value: 'low_shelf', label: 'Low Shelf', icon: 'M3 14h6l4 -6h8' },
    { value: 'high_shelf', label: 'High Shelf', icon: 'M3 8h8l4 6h6' },
    { value: 'low_pass', label: 'Low Pass', icon: 'M3 8h8l4 8h6' },
    { value: 'high_pass', label: 'High Pass', icon: 'M3 16h6l4 -8h8' },
    { value: 'band_pass', label: 'Band Pass', icon: 'M3 12h4l2 -6h2l2 12h2l2 -6h4' },
    { value: 'notch', label: 'Notch', icon: 'M3 12h6l2 8h2l2 -16h2l2 8h4' }
  ];

  let bands: EqBand[] = [
    { frequency: 60, gain_db: 0, q: 0.7, filter_type: 'low_shelf' },
    { frequency: 250, gain_db: 0, q: 1.4, filter_type: 'peaking' },
    { frequency: 1000, gain_db: 0, q: 1.4, filter_type: 'peaking' },
    { frequency: 4000, gain_db: 0, q: 1.4, filter_type: 'peaking' },
    { frequency: 12000, gain_db: 0, q: 0.7, filter_type: 'high_shelf' }
  ];
  
  let preamp = 0;
  let isEqEnabled = false;
  let graphCanvas: HTMLCanvasElement;
  
  // Interactive UI States
  let activeBandIndex = 0;
  let draggingBandIndex = -1;
  let isDragging = false;

  const minFreqLog = Math.log10(20);
  const maxFreqLog = Math.log10(20000);
  const dbRange = 24; // +12dB to -12dB

  function loadSettings() {
    isEqEnabled = localStorage.getItem('vesper_dsp_eq_manual_enabled') === 'true';
    const savedBands = localStorage.getItem('vesper_dsp_eq_manual_bands');
    const savedPreamp = localStorage.getItem('vesper_dsp_eq_manual_preamp');
    
    if (savedBands) {
      let parsedBands = JSON.parse(savedBands);
      // Migration: convert old q_value to q
      bands = parsedBands.map((b: any) => ({
        frequency: b.frequency,
        gain_db: b.gain_db,
        q: b.q !== undefined ? b.q : (b.q_value !== undefined ? b.q_value : 1.4),
        filter_type: b.filter_type
      }));
    }
    if (savedPreamp) preamp = parseFloat(savedPreamp);
    
    drawGraph();
  }

  function saveSettings() {
    localStorage.setItem('vesper_dsp_eq_manual_enabled', isEqEnabled.toString());
    localStorage.setItem('vesper_dsp_eq_manual_bands', JSON.stringify(bands));
    localStorage.setItem('vesper_dsp_eq_manual_preamp', preamp.toString());
  }

  let applyEqTimeout: number;

  async function applyEq() {
    saveSettings();
    if (isEqEnabled) {
      try {
        await invoke('apply_output_eq_profile', {
          profile: { enabled: true, preamp_gain: preamp, bands: bands }
        });
        emit('update-signal-path');
      } catch (e) {
        console.error("EQ 적용 실패", e);
      }
    } else {
      await invoke('apply_output_eq_profile', { profile: { enabled: false, preamp_gain: 0, bands: [] } });
      emit('update-signal-path');
    }
  }

  function applyEqDebounced() {
    clearTimeout(applyEqTimeout);
    applyEqTimeout = window.setTimeout(() => {
      applyEq();
    }, 50);
  }

  function addBand() {
    if (bands.length >= 10) return;
    bands = [...bands, { frequency: 1000, gain_db: 0, q: 1.0, filter_type: 'peaking' }];
    activeBandIndex = bands.length - 1;
    applyEq();
  }

  function removeBand(index: number) {
    if (bands.length <= 1) return;
    bands = bands.filter((_, i) => i !== index);
    if (activeBandIndex >= bands.length) activeBandIndex = Math.max(0, bands.length - 1);
    applyEq();
  }

  function freqToX(freq: number, width: number) {
    const logF = Math.log10(Math.max(20, Math.min(20000, freq)));
    return ((logF - minFreqLog) / (maxFreqLog - minFreqLog)) * width;
  }
  
  function xToFreq(x: number, width: number) {
    const logF = minFreqLog + (x / width) * (maxFreqLog - minFreqLog);
    return Math.pow(10, logF);
  }

  function gainToY(gain: number, height: number) {
    // 0dB가 중앙(height/2)
    // +12dB가 상단 (0), -12dB가 하단 (height)
    return height / 2 - (gain / dbRange) * height;
  }

  function yToGain(y: number, height: number) {
    return ((height / 2 - y) / height) * dbRange;
  }

  // --- Mouse Interactions ---
  function getMousePos(e: MouseEvent | TouchEvent) {
    const rect = graphCanvas.getBoundingClientRect();
    let clientX, clientY;
    if ('touches' in e) {
      clientX = e.touches[0].clientX;
      clientY = e.touches[0].clientY;
    } else {
      clientX = (e as MouseEvent).clientX;
      clientY = (e as MouseEvent).clientY;
    }
    return {
      x: clientX - rect.left,
      y: clientY - rect.top
    };
  }

  function handleMouseDown(e: MouseEvent | TouchEvent) {
    if (!isEqEnabled) return;
    const { x, y } = getMousePos(e);
    const width = graphCanvas.width;
    const height = graphCanvas.height;

    // Check hit for nodes
    let hitIndex = -1;
    let minDistance = 20; // Hit radius

    bands.forEach((band, i) => {
      const bx = freqToX(band.frequency, width);
      const by = gainToY(band.gain_db, height);
      const dist = Math.sqrt((bx - x) ** 2 + (by - y) ** 2);
      if (dist < minDistance) {
        minDistance = dist;
        hitIndex = i;
      }
    });

    if (hitIndex !== -1) {
      draggingBandIndex = hitIndex;
      activeBandIndex = hitIndex;
      isDragging = true;
    }
  }

  function handleMouseMove(e: MouseEvent | TouchEvent) {
    if (!isDragging || draggingBandIndex === -1 || !isEqEnabled) return;
    e.preventDefault();
    const { x, y } = getMousePos(e);
    const width = graphCanvas.width;
    const height = graphCanvas.height;

    // Constrain x and y
    const clampedX = Math.max(0, Math.min(width, x));
    const clampedY = Math.max(0, Math.min(height, y));

    bands[draggingBandIndex].frequency = parseFloat(xToFreq(clampedX, width).toFixed(1));
    bands[draggingBandIndex].gain_db = parseFloat(yToGain(clampedY, height).toFixed(1));
    
    // Bounds check
    bands[draggingBandIndex].frequency = Math.max(20, Math.min(20000, bands[draggingBandIndex].frequency));
    bands[draggingBandIndex].gain_db = Math.max(-12, Math.min(12, bands[draggingBandIndex].gain_db));
    
    bands = [...bands]; // trigger Svelte reactivity
  }

  function handleMouseUp() {
    if (isDragging) {
      isDragging = false;
      draggingBandIndex = -1;
      applyEqDebounced(); // 드래그 끝날 때만 백엔드에 반영
    }
  }

  // --- Rendering ---
  let animationFrameId: number;

  function drawGraph() {
    if (!graphCanvas) return;
    const ctx = graphCanvas.getContext('2d');
    if (!ctx) return;
    
    const width = graphCanvas.width;
    const height = graphCanvas.height;
    
    // 배경 클리어
    ctx.clearRect(0, 0, width, height);
    
    // Y축 그리드 (dB)
    ctx.strokeStyle = 'rgba(255,255,255,0.03)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let db = -12; db <= 12; db += 3) {
      const y = gainToY(db, height);
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.fillStyle = 'rgba(255,255,255,0.2)';
      ctx.font = '10px sans-serif';
      if (db !== 0 && db % 6 === 0) ctx.fillText(`${db > 0 ? '+' : ''}${db}`, 10, y - 5);
    }
    
    // X축 그리드 (Log Scale Freq)
    const freqs = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
    freqs.forEach(f => {
      const x = freqToX(f, width);
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      if (f === 100 || f === 1000 || f === 10000) {
        ctx.fillStyle = 'rgba(255,255,255,0.2)';
        ctx.fillText(`${f < 1000 ? f : f/1000+'k'}`, x + 5, height - 10);
      }
    });
    ctx.stroke();

    // 플랫(0dB) 기준선
    ctx.strokeStyle = 'rgba(255,255,255,0.1)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();

    // 1. 개별 밴드의 독립적인 곡선 그리기 (Individual Band Curves)
    if (isEqEnabled) {
      bands.forEach((band, bandIndex) => {
        const isActive = activeBandIndex === bandIndex;
        
        ctx.beginPath();
        // 활성 밴드는 초록색 반투명, 비활성 밴드는 회색 반투명
        ctx.strokeStyle = isActive ? 'rgba(34, 197, 94, 0.5)' : 'rgba(255, 255, 255, 0.15)';
        ctx.lineWidth = isActive ? 2 : 1;
        
        for (let x = 0; x <= width; x += 2) {
          const currentFreq = xToFreq(x, width);
          let bandGain = 0; // 프리앰프 제외, 해당 밴드만의 순수 게인
          
          if (band.gain_db !== 0 || band.filter_type.includes('pass') || band.filter_type === 'notch') {
            const ratio = currentFreq / band.frequency;
            const logRatio = Math.log10(ratio);
            const octaves = Math.log2(ratio);
            const spread = 0.5 / band.q; 
            const effect = Math.exp(-(logRatio * logRatio) / (spread * spread));

            if (band.filter_type === 'peaking') {
               bandGain += band.gain_db * effect;
            } else if (band.filter_type === 'low_shelf') {
               const shelfEffect = 1 / (1 + Math.pow(ratio, 2 * band.q));
               bandGain += band.gain_db * shelfEffect;
            } else if (band.filter_type === 'high_shelf') {
               const shelfEffect = 1 / (1 + Math.pow(1/ratio, 2 * band.q));
               bandGain += band.gain_db * shelfEffect;
            } else if (band.filter_type === 'low_pass') {
               if (currentFreq > band.frequency) bandGain -= (octaves * 12);
               if (Math.abs(octaves) < 0.2) bandGain += band.q * 2.0;
            } else if (band.filter_type === 'high_pass') {
               if (currentFreq < band.frequency) bandGain -= (-octaves * 12);
               if (Math.abs(octaves) < 0.2) bandGain += band.q * 2.0;
            } else if (band.filter_type === 'band_pass') {
               const bpEffect = 1 / (1 + Math.pow(logRatio * 3 * band.q, 2));
               bandGain += (bpEffect * 24) - 24;
            } else if (band.filter_type === 'notch') {
               const notchSpread = 0.1 / band.q;
               const notchEffect = Math.exp(-(logRatio * logRatio) / (notchSpread * notchSpread));
               bandGain -= 24 * notchEffect;
            }
          }
          
          bandGain = Math.max(-12, Math.min(12, bandGain));
          const y = gainToY(bandGain, height);
          
          if (x === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.stroke();
      });
    }

    // 2. 전체 합산 곡선 (Total Response Curve)
    ctx.strokeStyle = isEqEnabled ? '#22c55e' : 'rgba(255,255,255,0.1)';
    ctx.lineWidth = 3;
    ctx.beginPath();
    
    const points: {x: number, y: number}[] = [];
    
    for (let x = 0; x <= width; x += 2) {
      const currentFreq = xToFreq(x, width);
      let totalGain = preamp;
      
      bands.forEach(band => {
        if (band.gain_db !== 0 || band.filter_type.includes('pass') || band.filter_type === 'notch') {
          const ratio = currentFreq / band.frequency;
          const logRatio = Math.log10(ratio);
          const octaves = Math.log2(ratio);
          const spread = 0.5 / band.q; 
          const effect = Math.exp(-(logRatio * logRatio) / (spread * spread));

          if (band.filter_type === 'peaking') {
             totalGain += band.gain_db * effect;
          } else if (band.filter_type === 'low_shelf') {
             const shelfEffect = 1 / (1 + Math.pow(ratio, 2 * band.q));
             totalGain += band.gain_db * shelfEffect;
          } else if (band.filter_type === 'high_shelf') {
             const shelfEffect = 1 / (1 + Math.pow(1/ratio, 2 * band.q));
             totalGain += band.gain_db * shelfEffect;
          } else if (band.filter_type === 'low_pass') {
             if (currentFreq > band.frequency) totalGain -= (octaves * 12);
             if (Math.abs(octaves) < 0.2) totalGain += band.q * 2.0;
          } else if (band.filter_type === 'high_pass') {
             if (currentFreq < band.frequency) totalGain -= (-octaves * 12);
             if (Math.abs(octaves) < 0.2) totalGain += band.q * 2.0;
          } else if (band.filter_type === 'band_pass') {
             const bpEffect = 1 / (1 + Math.pow(logRatio * 3 * band.q, 2));
             totalGain += (bpEffect * 24) - 24;
          } else if (band.filter_type === 'notch') {
             const notchSpread = 0.1 / band.q;
             const notchEffect = Math.exp(-(logRatio * logRatio) / (notchSpread * notchSpread));
             totalGain -= 24 * notchEffect;
          }
        }
      });
      
      totalGain = Math.max(-12, Math.min(12, totalGain));
      const y = gainToY(totalGain, height);
      points.push({x, y});
    }

    if (points.length > 0) {
      ctx.moveTo(points[0].x, points[0].y);
      for (let i = 1; i < points.length; i++) {
        ctx.lineTo(points[i].x, points[i].y);
      }
      ctx.stroke();

      if (isEqEnabled) {
        ctx.lineTo(width, height);
        ctx.lineTo(0, height);
        ctx.closePath();
        const gradient = ctx.createLinearGradient(0, 0, 0, height);
        gradient.addColorStop(0, 'rgba(34, 197, 94, 0.25)'); // Green
        gradient.addColorStop(1, 'rgba(34, 197, 94, 0)');    // Fade out
        ctx.fillStyle = gradient;
        ctx.fill();
      }
    }
    
    // 3. 드래그 가능한 노드(점) 찍기
    if (isEqEnabled) {
      bands.forEach((band, i) => {
        const x = freqToX(band.frequency, width);
        const y = gainToY(band.gain_db, height);
        
        const isActive = activeBandIndex === i;
        const isDragged = draggingBandIndex === i;
        
        // Node 배경 원
        ctx.beginPath();
        ctx.arc(x, y, isActive ? 12 : 10, 0, Math.PI * 2);
        ctx.fillStyle = isActive ? '#22c55e' : '#1e1e24'; // Active: Green, Inactive: Dark
        ctx.fill();
        
        // Node 테두리
        ctx.lineWidth = 2;
        ctx.strokeStyle = isActive ? '#ffffff' : '#22c55e';
        ctx.stroke();
        
        // 텍스트 (밴드 번호)
        ctx.fillStyle = isActive ? '#ffffff' : '#ffffff';
        ctx.font = 'bold 10px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(`${i+1}`, x, y + 1);
      });
    }
  }

  function loop() {
    drawGraph();
    animationFrameId = requestAnimationFrame(loop);
  }

  onMount(() => {
    loadSettings();
    const resizeObserver = new ResizeObserver(() => drawGraph());
    if (graphCanvas) {
      resizeObserver.observe(graphCanvas);
      // Canvas 크기를 픽셀 밀도에 맞게 보정 (Sharpness)
      const rect = graphCanvas.getBoundingClientRect();
      graphCanvas.width = rect.width;
      graphCanvas.height = rect.height;
    }
    
    loop(); // Render loop 시작

    // 글로벌 마우스 업 처리 (캔버스 밖으로 드래그 시)
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('touchend', handleMouseUp);

    return () => {
      cancelAnimationFrame(animationFrameId);
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('touchend', handleMouseUp);
    };
  });

  $: if (bands || preamp || isEqEnabled) {
    if (typeof window !== 'undefined' && !isDragging) {
      // 드래그 중이 아닐 때 숫자를 입력하면 즉시 캔버스 업데이트
      applyEqDebounced();
    }
  }
</script>

<div class="relative w-screen h-screen bg-[#0e0e12] flex flex-col overflow-hidden select-none font-sans text-white/90">
  
  <!-- 창 헤더 (Roon Style) -->
  <div class="flex items-center justify-between px-6 py-4 bg-[#141419] border-b border-white/5 shadow-md" data-tauri-drag-region>
    <div class="flex items-center gap-4 pointer-events-none">
      <div class="w-9 h-9 rounded-full bg-gradient-to-br from-green-400 to-green-600 flex items-center justify-center shadow-lg">
        <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"></path></svg>
      </div>
      <div>
        <h1 class="text-[15px] font-bold tracking-wide">Parametric EQ</h1>
        <p class="text-[11px] text-white/40 font-medium tracking-wider">Precision Audio Control</p>
      </div>
    </div>
    
    <div class="flex items-center gap-5">
      <label class="flex items-center gap-2 cursor-pointer group">
        <span class="text-xs font-bold uppercase tracking-wider transition-colors {isEqEnabled ? 'text-green-400' : 'text-white/30'}">Enabled</span>
        <div class="relative">
          <input type="checkbox" bind:checked={isEqEnabled} class="sr-only" />
          <div class="block w-10 h-6 rounded-full transition-colors duration-300 {isEqEnabled ? 'bg-green-500' : 'bg-white/10 border border-white/10'}"></div>
          <div class="absolute left-1 top-1 bg-white w-4 h-4 rounded-full transition-transform duration-300 {isEqEnabled ? 'transform translate-x-4' : ''}"></div>
        </div>
      </label>
      <div class="w-px h-6 bg-white/10 mx-1"></div>
      <button on:click={() => getCurrentWindow().close()} class="w-8 h-8 rounded-full hover:bg-white/10 flex items-center justify-center transition-colors cursor-pointer text-white/50 hover:text-white">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
      </button>
    </div>
  </div>

  <!-- 메인 레이아웃 (70% Canvas, 30% Controls) -->
  <div class="flex flex-col flex-1 overflow-hidden">
    
    <!-- 인터랙티브 그래프 영역 (Roon Canvas) -->
    <div class="relative w-full flex-1 bg-gradient-to-b from-[#1a1a21] to-[#0e0e12] overflow-hidden">
      <!-- 캔버스는 마우스 이벤트 처리를 위해 꽉 채움 -->
      <canvas 
        bind:this={graphCanvas} 
        on:mousedown={handleMouseDown}
        on:mousemove={handleMouseMove}
        on:touchstart={handleMouseDown}
        on:touchmove={handleMouseMove}
        class="absolute inset-0 w-full h-full cursor-crosshair touch-none"
      ></canvas>
      
      {#if !isEqEnabled}
        <div class="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm pointer-events-none">
          <span class="px-4 py-2 rounded-full border border-white/10 bg-white/5 text-white/50 font-bold text-xs tracking-widest uppercase">EQ Bypass</span>
        </div>
      {/if}
    </div>

    <!-- 하단 상세 파라미터 컨트롤 (Roon Style Dashboard) -->
    <div class="h-64 shrink-0 bg-[#16161c] border-t border-white/5 flex flex-col">
      
      <!-- 상단 탭/툴바 -->
      <div class="flex items-center justify-between px-6 py-3 border-b border-white/5 bg-[#1a1a21]">
        <div class="flex items-center gap-2 overflow-x-auto no-scrollbar">
          {#each bands as band, i}
            <button 
              on:click={() => activeBandIndex = i}
              class="px-4 py-1.5 rounded-full text-xs font-bold transition-all border {activeBandIndex === i ? 'bg-green-500 border-green-400 text-white shadow-lg shadow-green-500/20' : 'bg-transparent border-white/10 text-white/50 hover:bg-white/5'}"
            >
              Band {i + 1}
            </button>
          {/each}
        </div>
        
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-3">
            <span class="text-[10px] font-bold text-white/40 uppercase tracking-widest">Preamp</span>
            <input type="number" bind:value={preamp} step="0.1" class="w-16 bg-[#0e0e12] border border-white/10 rounded-lg px-2 py-1 text-xs font-mono text-center text-white/90 outline-none focus:border-green-500 transition-colors" />
            <span class="text-[10px] text-white/40">dB</span>
          </div>
          <div class="w-px h-4 bg-white/10"></div>
          <button on:click={addBand} disabled={bands.length >= 10} class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/5 text-white/70 hover:bg-white/10 hover:text-white transition-colors border border-white/5 disabled:opacity-30">
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
            <span class="text-[10px] font-bold uppercase tracking-wider">Add</span>
          </button>
        </div>
      </div>

      <!-- 활성 밴드 상세 컨트롤 폼 -->
      {#if bands[activeBandIndex]}
        <div class="flex-1 flex items-center justify-center p-6 bg-[#0e0e12]">
          <div class="w-full max-w-4xl grid grid-cols-5 gap-6">
            
            <!-- Type Selector -->
            <div class="col-span-2 flex flex-col gap-2">
              <label class="text-[10px] font-bold text-white/40 uppercase tracking-widest">Filter Type</label>
              <div class="grid grid-cols-4 gap-2">
                {#each filterTypes as ft}
                  <button 
                    on:click={() => bands[activeBandIndex].filter_type = ft.value}
                    class="flex flex-col items-center gap-1.5 p-2 rounded-xl border transition-all {bands[activeBandIndex].filter_type === ft.value ? 'bg-green-500/10 border-green-500 text-green-400' : 'bg-[#16161c] border-white/5 text-white/40 hover:bg-white/5 hover:text-white/70'}"
                    title={ft.label}
                  >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={ft.icon}></path></svg>
                    <span class="text-[9px] font-bold whitespace-nowrap">{ft.label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <!-- Freq -->
            <div class="flex flex-col gap-3 justify-center">
              <label class="text-[10px] font-bold text-white/40 uppercase tracking-widest">Frequency</label>
              <div class="flex items-end gap-2">
                <input type="number" bind:value={bands[activeBandIndex].frequency} min="20" max="20000" class="w-full bg-[#16161c] border border-white/10 rounded-xl px-4 py-3 text-lg font-mono text-white outline-none focus:border-green-500 transition-colors" />
                <span class="text-xs text-white/40 font-bold mb-3">Hz</span>
              </div>
            </div>

            <!-- Gain -->
            <div class="flex flex-col gap-3 justify-center">
              <label class="text-[10px] font-bold text-white/40 uppercase tracking-widest">Gain</label>
              <div class="flex items-end gap-2">
                <input type="number" bind:value={bands[activeBandIndex].gain_db} step="0.1" class="w-full bg-[#16161c] border border-white/10 rounded-xl px-4 py-3 text-lg font-mono text-white outline-none focus:border-green-500 transition-colors" />
                <span class="text-xs text-white/40 font-bold mb-3">dB</span>
              </div>
            </div>

            <!-- Q Factor -->
            <div class="flex flex-col gap-3 justify-center relative">
              <label class="text-[10px] font-bold text-white/40 uppercase tracking-widest">Q Factor (Width)</label>
              <div class="flex items-end gap-2">
                <input type="number" bind:value={bands[activeBandIndex].q} step="0.1" min="0.1" max="10" class="w-full bg-[#16161c] border border-white/10 rounded-xl px-4 py-3 text-lg font-mono text-white outline-none focus:border-green-500 transition-colors" />
              </div>
              <button on:click={() => removeBand(activeBandIndex)} class="absolute -top-2 right-0 text-[10px] font-bold text-red-500/70 hover:text-red-400 uppercase tracking-widest hover:bg-red-500/10 px-2 py-1 rounded-md transition-colors">
                Remove Band
              </button>
            </div>
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  /* 컴팩트 스크롤바 */
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;  /* IE and Edge */
    scrollbar-width: none;  /* Firefox */
  }
  
  /* Number input arrow 숨기기 */
  input[type=number]::-webkit-inner-spin-button, 
  input[type=number]::-webkit-outer-spin-button { 
    -webkit-appearance: none; 
    margin: 0; 
  }
</style>
