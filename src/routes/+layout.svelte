<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { attachGlobalSoundListener } from '$lib/sound';

  let { children } = $props();

  onMount(() => {
    const detach = attachGlobalSoundListener();
    return () => {
      detach();
    };
  });
</script>

{@render children()}
