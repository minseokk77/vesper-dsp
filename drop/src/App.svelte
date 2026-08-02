<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import QRCode from 'qrcode';

  type DeviceInfo = {
    id: string;
    name: string;
    ip: string;
    port: number;
    os: string;
    lastSeen: number;
  };

  type TrustedDevice = { id: string; name: string };
  type AppSettings = {
    deviceId: string;
    deviceName: string;
    pairCode: string;
    trustedDevices: TrustedDevice[];
    receiveDirectory: string | null;
    maxFileSize: number;
  };
  type PairingPayload = { uri: string; code: string; ip: string };
  type PairCandidate = { id: string; name: string; ip: string };
  type IncomingRequest = {
    transferId: string;
    deviceId: string;
    deviceName: string;
    fileName: string;
    fileSize: number;
  };
  type TransferProgress = {
    transferId: string;
    direction: 'send' | 'receive';
    fileName: string;
    bytesTransferred: number;
    totalBytes: number;
    currentMibps: number;
    averageMibps: number;
    peakMibps: number;
    progressPercent: number;
    etaSeconds: number;
    itemIndex: number;
    itemCount: number;
  };
  type TransferCompleted = {
    transferId: string;
    direction: 'send' | 'receive';
    fileName: string;
    savedPath: string | null;
    totalBytes: number;
    averageMibps: number;
    sha256: string;
    itemIndex: number;
    itemCount: number;
  };
  type HistoryItem = TransferCompleted & { completedAt: number };
  type TransferStatus = 'idle' | 'active' | 'completed' | 'error';
  type LastAction = { kind: 'files'; device: DeviceInfo; paths: string[] };

  let devices = $state<DeviceInfo[]>([]);
  let selectedDevice = $state<DeviceInfo | null>(null);
  let transfer = $state<TransferProgress | null>(null);
  let transferStatus = $state<TransferStatus>('idle');
  let completedTransfer = $state<TransferCompleted | null>(null);
  let incomingRequest = $state<IncomingRequest | null>(null);
  let settings = $state<AppSettings | null>(null);
  let pairing = $state<PairingPayload | null>(null);
  let pairingQr = $state('');
  let pairCodeInput = $state('');
  let qrInput = $state('');
  let deviceNameInput = $state('');
  let history = $state<HistoryItem[]>([]);
  let lastAction = $state<LastAction | null>(null);
  let notice = $state('같은 LAN의 기기를 자동으로 찾고 있습니다.');
  let noticeTitle = $state('Vesper Drop 준비 완료');
  let errorMessage = $state('');
  let isBusy = $state(false);
  let settingsOpen = $state(false);
  let pairingOpen = $state(false);
  let historyOpen = $state(false);
  let dragActive = $state(false);
  let autostartEnabled = $state(false);
  let backgroundEnabled = $state(true);

  const transferTitle = $derived.by(() => {
    if (!transfer) return '전송 모니터';
    if (transferStatus === 'completed') return transfer.direction === 'send' ? '전송 완료' : '수신 완료';
    if (transferStatus === 'error') return '전송 오류';
    return transfer.direction === 'send' ? '보내는 중' : '받는 중';
  });
  const isAndroid = $derived(/Android/i.test(navigator.userAgent));
  const trustedIds = $derived(new Set(settings?.trustedDevices.map((device) => device.id) ?? []));

  onMount(() => {
    const unlisteners: UnlistenFn[] = [];
    let disposed = false;
    history = loadHistory();
    const staleTimer = window.setInterval(() => {
      const cutoff = Date.now() - 7_000;
      devices = devices.filter((device) => device.lastSeen >= cutoff);
      if (selectedDevice && !devices.some((device) => device.id === selectedDevice?.id)) selectedDevice = null;
    }, 2_000);

    async function subscribe() {
      await invoke('request_local_network_access');
      await refreshSettings();
      autostartEnabled = await invoke<boolean>('get_autostart_enabled');
      if (isAndroid) await invoke('set_background_receive', { enabled: backgroundEnabled });
      unlisteners.push(
        await listen<Omit<DeviceInfo, 'lastSeen'>>('device-discovered', ({ payload }) => {
          const discovered = { ...payload, lastSeen: Date.now() };
          devices = [...devices.filter((device) => device.id !== discovered.id), discovered]
            .sort((left, right) => left.id.localeCompare(right.id));
          notice = `${discovered.name} 기기를 찾았습니다.`;
        }),
        await listen<TransferProgress>('transfer-progress', ({ payload }) => {
          transfer = payload;
          transferStatus = 'active';
          isBusy = true;
        }),
        await listen<TransferCompleted>('transfer-completed', ({ payload }) => {
          completedTransfer = payload;
          transferStatus = 'completed';
          transfer = transfer
            ? { ...transfer, progressPercent: 100, etaSeconds: 0, averageMibps: payload.averageMibps }
            : null;
          isBusy = payload.itemIndex < payload.itemCount;
          addHistory(payload);
          notice = `${payload.fileName} ${payload.direction === 'receive' ? '수신' : '전송'} 완료 · SHA-256 확인됨`;
        }),
        await listen<IncomingRequest>('incoming-request', ({ payload }) => {
          incomingRequest = payload;
          notice = `${payload.deviceName}에서 수신 승인을 기다리고 있습니다.`;
        }),
        await listen<string>('device-paired', async ({ payload }) => {
          noticeTitle = '페어링 완료';
          notice = `${payload} 기기와 페어링했습니다.`;
          pairingOpen = false;
          await refreshSettings();
        }),
        await listen<string>('transfer-error', ({ payload }) => {
          isBusy = false;
          transferStatus = 'error';
          errorMessage = payload;
        }),
        await listen<string>('network-error', ({ payload }) => (errorMessage = payload)),
        await listen('app-backgrounded', resetIfFinished),
        await listen('app-foregrounded', resetIfFinished),
        await getCurrentWebviewWindow().onDragDropEvent(({ payload }) => {
          if (payload.type === 'enter' || payload.type === 'over') dragActive = true;
          if (payload.type === 'leave') dragActive = false;
          if (payload.type === 'drop') {
            dragActive = false;
            if (payload.paths.length > 0) void sendDropped(payload.paths);
          }
        }),
      );
      if (disposed) unlisteners.splice(0).forEach((unlisten) => unlisten());
    }
    void subscribe().catch((error: unknown) => (errorMessage = readableError(error)));
    return () => {
      disposed = true;
      window.clearInterval(staleTimer);
      unlisteners.splice(0).forEach((unlisten) => unlisten());
    };
  });

  async function refreshSettings() {
    settings = await invoke<AppSettings>('get_app_settings');
    deviceNameInput = settings.deviceName;
    pairing = await invoke<PairingPayload>('get_pairing_payload');
    pairingQr = await QRCode.toDataURL(pairing.uri, { margin: 1, width: 220, color: { dark: '#11152d', light: '#ffffff' } });
  }

  async function chooseFiles(device: DeviceInfo) {
    if (isBusy) return;
    const selected = await open({ multiple: true, directory: false });
    const paths = typeof selected === 'string' ? [selected] : selected;
    if (paths?.length) await sendPaths(device, paths);
  }

  async function chooseFolder(device: DeviceInfo) {
    if (isBusy) return;
    if (isAndroid) {
      try {
        const selected = await invoke<string>('pick_folder');
        if (selected) await sendPaths(device, [selected]);
      } catch (error: unknown) {
        const message = readableError(error);
        if (!message.includes('취소')) errorMessage = message;
      }
      return;
    }
    const selected = await open({ multiple: false, directory: true });
    if (typeof selected === 'string') await sendPaths(device, [selected]);
  }

  async function sendPaths(device: DeviceInfo, paths: string[]) {
    selectedDevice = device;
    lastAction = { kind: 'files', device, paths };
    await runTransfer(() => invoke('send_files', { targetIp: device.ip, filePaths: paths }), `${paths.length}개 항목을 ${device.name}에 전송합니다.`);
  }

  async function runTransfer(action: () => Promise<unknown>, message: string) {
    errorMessage = '';
    completedTransfer = null;
    transferStatus = 'active';
    isBusy = true;
    notice = message;
    try {
      await action();
      isBusy = false;
    } catch (error: unknown) {
      isBusy = false;
      transferStatus = 'error';
      errorMessage = readableError(error);
    }
  }

  async function sendDropped(paths: string[]) {
    const target = selectedDevice ?? (devices.length === 1 ? devices[0] : null);
    if (!target) {
      errorMessage = '먼저 전송할 기기를 선택해 주세요.';
      return;
    }
    await sendPaths(target, paths);
  }

  async function retryLast() {
    if (!lastAction || isBusy) return;
    await sendPaths(lastAction.device, lastAction.paths);
  }

  async function cancelCurrent() {
    await invoke('cancel_transfer');
    notice = '전송 취소를 요청했습니다.';
  }

  async function answerIncoming(accept: boolean, remember: boolean) {
    if (!incomingRequest) return;
    const transferId = incomingRequest.transferId;
    incomingRequest = null;
    await invoke('respond_to_incoming', { transferId, accept, remember });
    if (remember) await refreshSettings();
  }

  async function pairWithCode() {
    errorMessage = '';
    try {
      const candidates: PairCandidate[] = devices.map(({ id, name, ip }) => ({ id, name, ip }));
      const paired = await invoke<TrustedDevice>('pair_by_code', { code: pairCodeInput, candidates });
      pairCodeInput = '';
      noticeTitle = '페어링 완료';
      notice = `${paired.name} 기기와 페어링했습니다.`;
      pairingOpen = false;
      await refreshSettings();
    } catch (error: unknown) {
      errorMessage = readableError(error);
    }
  }

  async function pairWithQrData(data = qrInput) {
    errorMessage = '';
    try {
      if (!data.trim()) throw new Error('Vesper Drop QR 코드를 스캔하거나 데이터를 입력하세요.');
      const url = new URL(data.trim());
      if (url.protocol !== 'vesperdrop:' || url.hostname !== 'lan-pair') throw new Error('Vesper Drop LAN QR 데이터가 아닙니다.');
      const candidate: PairCandidate = {
        id: requiredParam(url, 'deviceId'),
        name: requiredParam(url, 'name'),
        ip: requiredParam(url, 'ip'),
      };
      const paired = await invoke<TrustedDevice>('pair_from_qr', { candidate, code: requiredParam(url, 'code') });
      qrInput = '';
      noticeTitle = '페어링 완료';
      notice = `${paired.name} 기기와 QR 페어링했습니다.`;
      pairingOpen = false;
      await refreshSettings();
    } catch (error: unknown) {
      errorMessage = readableError(error);
    }
  }

  async function scanQrAndPair() {
    errorMessage = '';
    try {
      const data = await invoke<string>('scan_pairing_qr');
      qrInput = data;
      await pairWithQrData(data);
    } catch (error: unknown) {
      errorMessage = readableError(error);
    }
  }

  async function saveDeviceName() {
    await invoke('set_device_name', { name: deviceNameInput });
    await refreshSettings();
    notice = '기기 이름을 변경했습니다.';
  }

  async function chooseReceiveDirectory() {
    const selected = await open({ multiple: false, directory: true });
    if (typeof selected !== 'string') return;
    await invoke('set_receive_directory', { path: selected });
    await refreshSettings();
  }

  async function resetReceiveDirectory() {
    await invoke('set_receive_directory', { path: null });
    await refreshSettings();
  }

  async function forgetTrusted(deviceId: string) {
    await invoke('forget_device', { deviceId });
    await refreshSettings();
  }

  async function toggleAutostart() {
    autostartEnabled = !autostartEnabled;
    try {
      await invoke('set_autostart_enabled', { enabled: autostartEnabled });
    } catch (error: unknown) {
      autostartEnabled = !autostartEnabled;
      errorMessage = readableError(error);
    }
  }

  async function toggleBackground() {
    backgroundEnabled = !backgroundEnabled;
    try {
      await invoke('set_background_receive', { enabled: backgroundEnabled });
    } catch (error: unknown) {
      backgroundEnabled = !backgroundEnabled;
      errorMessage = readableError(error);
    }
  }

  async function openReceivedFolder() {
    try {
      await invoke('open_received_folder');
    } catch (error: unknown) {
      errorMessage = readableError(error);
    }
  }

  function addHistory(payload: TransferCompleted) {
    history = [{ ...payload, completedAt: Date.now() }, ...history].slice(0, 100);
    localStorage.setItem('cross-drop-history', JSON.stringify(history));
  }

  function loadHistory(): HistoryItem[] {
    try {
      return JSON.parse(localStorage.getItem('cross-drop-history') ?? '[]') as HistoryItem[];
    } catch {
      return [];
    }
  }

  function clearHistory() {
    history = [];
    localStorage.removeItem('cross-drop-history');
  }

  function resetIfFinished() {
    if (transferStatus !== 'active') resetTransferUi();
  }

  function resetTransferUi() {
    transfer = null;
    transferStatus = 'idle';
    completedTransfer = null;
    isBusy = false;
    errorMessage = '';
    notice = '같은 LAN의 기기를 자동으로 찾고 있습니다.';
    noticeTitle = 'Vesper Drop 준비 완료';
  }

  function requiredParam(url: URL, name: string) {
    const value = url.searchParams.get(name);
    if (!value) throw new Error(`QR 데이터에 ${name} 값이 없습니다.`);
    return value;
  }

  function readableError(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  function formatBytes(bytes: number) {
    if (bytes < 1_024) return `${bytes} B`;
    if (bytes < 1_048_576) return `${(bytes / 1_024).toFixed(1)} KiB`;
    if (bytes < 1_073_741_824) return `${(bytes / 1_048_576).toFixed(1)} MiB`;
    return `${(bytes / 1_073_741_824).toFixed(2)} GiB`;
  }

  function formatEta(seconds: number) {
    if (seconds <= 0) return '곧 완료';
    if (seconds < 60) return `${seconds}초`;
    return `${Math.floor(seconds / 60)}분 ${seconds % 60}초`;
  }
</script>

<svelte:head>
  <title>Vesper Drop · LAN 파일 전송</title>
  <meta name="description" content="같은 LAN의 기기로 빠르고 안전하게 파일을 전송합니다." />
</svelte:head>

<main class:drag-active={dragActive}>
  <div class="ambient ambient-one"></div><div class="ambient ambient-two"></div>
  <div class="shell">
    <header class="topbar">
      <div class="brand-mark" aria-hidden="true"><span></span><span></span></div>
      <div class="brand-copy"><h1>Vesper Drop</h1><p>클라우드 없이, 같은 LAN에서 바로.</p></div>
      <div class="header-actions">
        <button onclick={() => (historyOpen = true)}>기록 <b>{history.length}</b></button>
        <button onclick={() => (pairingOpen = true)}>페어링</button>
        <button onclick={() => (settingsOpen = true)}>설정</button>
      </div>
      <div class="online-pill"><i></i> 수신 대기 중</div>
    </header>

    <section class="hero-grid">
      <article class="glass radar-card">
        <div class="section-heading"><div><span class="eyebrow">NEARBY</span><h2>주변 기기</h2></div><span class="device-count">{devices.length}</span></div>
        <div class="radar" class:has-devices={devices.length > 0}>
          <div class="radar-sweep"></div><div class="radar-ring ring-one"></div><div class="radar-ring ring-two"></div><div class="radar-ring ring-three"></div>
          <div class="radar-core"><div class="core-logo">C</div></div>
          {#each devices as device, index (device.id)}
            <button class="device-orb" class:selected={selectedDevice?.id === device.id} style={`--angle: ${index * 137 + 28}deg; --radius: ${index % 2 === 0 ? 108 : 148}px`} disabled={isBusy} onclick={() => (selectedDevice = device)} aria-label={`${device.name} 선택`}>
              <span class="device-icon">{device.os === 'android' ? 'M' : 'PC'}</span><strong>{device.name}</strong><small>{trustedIds.has(device.id) ? '신뢰됨' : device.ip}</small>
            </button>
          {/each}
          {#if devices.length === 0}<div class="searching-copy"><span></span> 기기 검색 중</div>{/if}
        </div>
        {#if selectedDevice}
          <div class="send-actions">
            <button onclick={() => chooseFiles(selectedDevice!)}>파일</button>
            <button onclick={() => chooseFolder(selectedDevice!)}>폴더</button>
          </div>
        {:else}<p class="hint">기기를 선택하거나 파일을 창에 끌어다 놓으세요.</p>{/if}
      </article>

      <aside class="side-stack">
        <article class="glass transfer-card" class:active={transfer !== null}>
          <div class="section-heading"><div><span class="eyebrow">TRANSFER</span><h2>{transferTitle}</h2></div><span class="direction-icon">{transfer?.direction === 'receive' ? '↓' : '↑'}</span></div>
          {#if transfer}
            <div class="file-row"><div class="file-icon">FILE</div><div><strong title={transfer.fileName}>{transfer.fileName}</strong><small>{formatBytes(transfer.totalBytes)} · {transfer.itemIndex}/{transfer.itemCount}</small></div></div>
            <div class="speed-readout"><strong>{transfer.currentMibps.toFixed(1)}</strong><span>MiB/s</span></div>
            <div class="progress-track"><i style={`width: ${transfer.progressPercent}%`}></i></div>
            <div class="progress-meta"><span>{transfer.progressPercent.toFixed(1)}%</span><span>{transferStatus === 'completed' ? '체크섬 확인 완료' : formatEta(transfer.etaSeconds)}</span></div>
            <div class="speed-stats"><div><small>전체 평균</small><strong>{transfer.averageMibps.toFixed(1)} MiB/s</strong></div><div><small>구간 최고</small><strong>{transfer.peakMibps.toFixed(1)} MiB/s</strong></div></div>
            <div class="transfer-actions">
              {#if transferStatus === 'active'}<button class="danger" onclick={cancelCurrent}>취소</button>{/if}
              {#if transferStatus === 'error' && lastAction}<button onclick={retryLast}>다시 시도</button>{/if}
              {#if transferStatus === 'completed' && completedTransfer?.direction === 'receive'}<button onclick={openReceivedFolder}>수신 폴더</button>{/if}
            </div>
          {:else}
            <div class="empty-transfer"><div class="empty-wave"><i></i><i></i><i></i><i></i><i></i></div><strong>아직 전송이 없습니다</strong><span>파일·폴더 선택 또는 드래그 앤 드롭을 사용하세요.</span></div>
          {/if}
        </article>
        <article class="glass status-card"><span class="status-dot"></span><div><strong>{errorMessage ? '확인이 필요합니다' : noticeTitle}</strong><p>{errorMessage || notice}</p></div>{#if errorMessage}<button onclick={() => (errorMessage = '')}>닫기</button>{/if}</article>
      </aside>
    </section>
    <footer><span>신뢰 기기 · SHA-256 검증 · LAN 전용</span><span>TCP · 1 MiB chunks · 10 Hz metrics</span></footer>
  </div>

  {#if dragActive}<div class="drop-overlay"><strong>여기에 놓아 전송</strong><span>{selectedDevice?.name ?? '기기를 먼저 선택하세요'}</span></div>{/if}

  {#if incomingRequest}
    <div class="modal-backdrop"><section class="glass modal compact"><span class="eyebrow">INCOMING</span><h2>{incomingRequest.deviceName}의 수신 요청</h2><p class="modal-file">{incomingRequest.fileName}<small>{formatBytes(incomingRequest.fileSize)}</small></p><div class="modal-actions three"><button class="ghost" onclick={() => answerIncoming(false, false)}>거절</button><button onclick={() => answerIncoming(true, false)}>이번만 허용</button><button class="primary" onclick={() => answerIncoming(true, true)}>신뢰하고 허용</button></div></section></div>
  {/if}

  {#if pairingOpen}
    <div class="modal-backdrop" role="presentation" onclick={(event) => event.currentTarget === event.target && (pairingOpen = false)}><section class="glass modal"><header><div><span class="eyebrow">LAN PAIRING</span><h2>기기 페어링</h2></div><button class="close" onclick={() => (pairingOpen = false)}>×</button></header><div class="pair-grid"><div class="qr-panel">{#if pairingQr}<img src={pairingQr} alt="Vesper Drop LAN 페어링 QR" />{/if}<strong>{pairing?.code}</strong><small>{pairing?.ip}</small></div><div class="pair-forms">{#if isAndroid}<button class="primary scan-button" onclick={scanQrAndPair}>카메라로 컴퓨터 QR 스캔</button>{/if}<label>상대 기기의 6자리 코드<input maxlength="6" inputmode="numeric" bind:value={pairCodeInput} placeholder="000000" /></label><button class="primary" onclick={pairWithCode} disabled={devices.length === 0}>코드로 연결</button><label>스캔한 Vesper Drop QR 데이터<textarea bind:value={qrInput} placeholder="vesperdrop://lan-pair?..." rows="3"></textarea></label><div class="inline-actions"><button class="primary" onclick={() => pairWithQrData()}>QR로 연결</button></div></div></div></section></div>
  {/if}

  {#if settingsOpen}
    <div class="modal-backdrop" role="presentation" onclick={(event) => event.currentTarget === event.target && (settingsOpen = false)}><section class="glass modal"><header><div><span class="eyebrow">SETTINGS</span><h2>Vesper Drop 설정</h2></div><button class="close" onclick={() => (settingsOpen = false)}>×</button></header><div class="settings-list"><label>이 기기 이름<div class="field-row"><input bind:value={deviceNameInput} maxlength="48" /><button onclick={saveDeviceName}>저장</button></div></label>{#if !isAndroid}<label>수신 폴더<div class="field-row"><input readonly value={settings?.receiveDirectory ?? '다운로드/Vesper Drop'} /><button onclick={chooseReceiveDirectory}>변경</button><button onclick={resetReceiveDirectory}>초기화</button></div></label><button class="toggle" class:on={autostartEnabled} onclick={toggleAutostart}><i></i><span>Windows 로그인 시 트레이로 자동 시작</span></button>{:else}<button class="toggle" class:on={backgroundEnabled} onclick={toggleBackground}><i></i><span>알림을 표시하고 백그라운드 수신 유지</span></button><p class="setting-note">Android 수신 위치: Download/Vesper Drop</p>{/if}<div class="trusted-list"><h3>신뢰 기기</h3>{#each settings?.trustedDevices ?? [] as device (device.id)}<div><span>{device.name}<small>{device.id.slice(0, 8)}</small></span><button onclick={() => forgetTrusted(device.id)}>삭제</button></div>{:else}<p>등록된 신뢰 기기가 없습니다.</p>{/each}</div></div></section></div>
  {/if}

  {#if historyOpen}
    <div class="modal-backdrop" role="presentation" onclick={(event) => event.currentTarget === event.target && (historyOpen = false)}><section class="glass modal"><header><div><span class="eyebrow">HISTORY</span><h2>전송 기록</h2></div><div class="inline-actions"><button onclick={clearHistory}>기록 지우기</button><button class="close" onclick={() => (historyOpen = false)}>×</button></div></header><div class="history-list">{#each history as item (item.transferId + item.completedAt)}<article><span class:receive={item.direction === 'receive'}>{item.direction === 'receive' ? '↓' : '↑'}</span><div><strong>{item.fileName}</strong><small>{new Date(item.completedAt).toLocaleString()} · {formatBytes(item.totalBytes)} · {item.averageMibps.toFixed(1)} MiB/s</small><code>{item.sha256.slice(0, 16)}…</code></div></article>{:else}<p>아직 완료된 전송이 없습니다.</p>{/each}</div></section></div>
  {/if}
</main>
