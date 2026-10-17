<script lang="ts">
  import PinView from '$lib/components/PinView.svelte';
  let { data } = $props();
</script>

<PinView pinId={data.pinId} />
