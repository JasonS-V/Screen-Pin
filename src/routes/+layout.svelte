<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { attachGlobalSoundListener } from '$lib/sound';
  import { i18n } from '$lib/i18n.svelte';

  let { children } = $props();

  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.lang = i18n.current;
    }
  });

  onMount(() => {
    const detach = attachGlobalSoundListener();
    return () => {
      detach();
    };
  });
</script>

{@render children()}
