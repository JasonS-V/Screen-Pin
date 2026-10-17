<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { isTauri } from '$lib/desktop';
  import OverlayView from '$lib/components/OverlayView.svelte';
  import PinView from '$lib/components/PinView.svelte';
  import MemoryView from '$lib/components/MemoryView.svelte';
  import CompanionView from '$lib/components/CompanionView.svelte';

  let currentView = $state<'companion' | 'overlay' | 'pin' | 'pin-toolbar' | 'memory'>('companion');
  let monitorId = $state<number>(0);
  let pinId = $state<string>('');

  onMount(() => {
    try {
      const label = isTauri() ? getCurrentWindow().label : '';
      const params = new URLSearchParams(window.location.search);
      const viewParam = params.get('view');

      if (label.startsWith('overlay-') || viewParam === 'overlay') {
        currentView = 'overlay';
        const rawId = params.get('monitor_id') || label.replace('overlay-', '');
        monitorId = parseInt(rawId, 10) || 0;
      } else if (label.startsWith('pin-toolbar-') || viewParam === 'pin-toolbar') {
        currentView = 'pin-toolbar';
        pinId = params.get('pin_id') || label.replace('pin-toolbar-', '');
      } else if (label.startsWith('pin-') || viewParam === 'pin') {
        currentView = 'pin';
        pinId = params.get('pin_id') || label.replace('pin-', '');
      } else if (label === 'memory-drawer' || viewParam === 'memory') {
        currentView = 'memory';
      } else {
        currentView = 'companion';
      }
    } catch (e) {
      console.warn('Error detectando ventana:', e);
    }
  });
</script>

{#if currentView === 'overlay'}
  <OverlayView {monitorId} />
{:else if currentView === 'pin'}
  <PinView {pinId} />
{:else if currentView === 'pin-toolbar'}
  <PinView {pinId} toolbar />
{:else if currentView === 'memory'}
  <MemoryView />
{:else}
  <CompanionView />
{/if}
