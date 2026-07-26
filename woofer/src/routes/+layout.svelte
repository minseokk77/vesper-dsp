<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  onMount(async () => {
    // Tauri 다중 창 버그(항상 메인 화면 렌더링) 해결을 위한 강제 라우팅
    const windowLabel = getCurrentWindow().label;
    if (windowLabel === 'signal_path') {
      goto('/signal');
    } else if (windowLabel.startsWith('dsp_settings')) {
      goto('/dsp');
    }

    let unlisten_min: () => void;
    
    listen('window-minimized', () => {
      // 메인 창이 트레이로 내려갈 때만 메인 화면으로 리셋 (팝업 창은 무시)
      if (windowLabel === 'main') {
        goto('/');
      }
    }).then(f => unlisten_min = f);

    return () => {
      if (unlisten_min) unlisten_min();
    };
  });
</script>

<div class="h-screen w-screen bg-transparent overflow-hidden">
  <slot />
</div>
