<script lang="ts">
  import { onMount } from 'svelte';
  import OverlayView from '$lib/components/OverlayView.svelte';
  let monitorId = $state(0);
  onMount(() => { monitorId = Number(new URLSearchParams(location.search).get('monitor_id')) || 0; });
</script>

<OverlayView {monitorId} />
