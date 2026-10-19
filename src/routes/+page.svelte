<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { isTauri } from '$lib/desktop';
  import OverlayView from '$lib/components/OverlayView.svelte';
  import PinView from '$lib/components/PinView.svelte';
  import MemoryView from '$lib/components/MemoryView.svelte';
  import CompanionView from '$lib/components/CompanionView.svelte';

  type ViewType = 'companion' | 'overlay' | 'pin' | 'pin-toolbar' | 'memory';

  function resolveInitialState(): { view: ViewType; monitorId: number; pinId: string } {
    if (typeof window === 'undefined') {
      return { view: 'companion', monitorId: 0, pinId: '' };
    }
    try {
      const label = isTauri() ? getCurrentWindow().label : '';
      const params = new URLSearchParams(window.location.search);
      const viewParam = params.get('view');

      if (label.startsWith('overlay-') || viewParam === 'overlay') {
        const rawId = params.get('monitor_id') || label.replace('overlay-', '');
        return { view: 'overlay', monitorId: parseInt(rawId, 10) || 0, pinId: '' };
      } else if (label.startsWith('pin-toolbar-') || viewParam === 'pin-toolbar') {
        const id = params.get('pin_id') || label.replace('pin-toolbar-', '');
        return { view: 'pin-toolbar', monitorId: 0, pinId: id };
      } else if (label.startsWith('pin-') || viewParam === 'pin') {
        const id = params.get('pin_id') || label.replace('pin-', '');
        return { view: 'pin', monitorId: 0, pinId: id };
      } else if (label === 'memory-drawer' || viewParam === 'memory') {
        return { view: 'memory', monitorId: 0, pinId: '' };
      }
    } catch (e) {
      console.warn('Error detectando ventana inicial:', e);
    }
    return { view: 'companion', monitorId: 0, pinId: '' };
  }

  const initial = resolveInitialState();
  let currentView = $state<ViewType>(initial.view);
  let monitorId = $state<number>(initial.monitorId);
  let pinId = $state<string>(initial.pinId);

  onMount(() => {
    const updated = resolveInitialState();
    currentView = updated.view;
    monitorId = updated.monitorId;
    pinId = updated.pinId;
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
