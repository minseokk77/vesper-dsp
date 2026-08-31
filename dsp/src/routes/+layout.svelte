<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  onMount(() => {
    const windowLabel = getCurrentWindow().label;
    
    // Tauri 다중 창 라우팅 픽스
    if (windowLabel === 'signal_path') {
      goto('/signal', { replaceState: true });
    } else if (windowLabel === 'parametric-eq') {
      goto('/parametric-eq', { replaceState: true });
    }

    let unlistenMin: (() => void) | undefined;
    void listen('window-minimized', () => {
      if (windowLabel === 'main') {
        // do something if needed
      }
    }).then((unlisten) => (unlistenMin = unlisten));

    return () => {
      unlistenMin?.();
    };
  });
</script>

<div class="h-screen w-screen bg-[#0c0c0e] overflow-hidden">
  <slot />
</div>
