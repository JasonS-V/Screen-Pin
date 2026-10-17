<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { isTauri, errorMessage } from '$lib/desktop';

  const desktop = isTauri();
  let busy = $state(false);
  let error = $state('');
  let shortcutWarnings = $state<string[]>([]);
  onMount(() => {
    if (!desktop) return;
    invoke<string[]>('get_shortcut_warnings').then(value => { shortcutWarnings = value; }).catch(cause => { error = errorMessage(cause); });
    const unlisten = listen<string>('app:error', event => { error = event.payload; });
    return () => { void unlisten.then(fn => fn()); };
  });

  async function run(command: string) {
    if (busy || !desktop) return;
    busy = true;
    error = '';
    try { await invoke(command); }
    catch (cause) { error = errorMessage(cause); }
    finally { busy = false; }
  }
</script>

<svelte:head><title>Screen-Pin · Atajos de teclado</title></svelte:head>

<main class="companion-shell">
  <header class="brand-row">
    <div class="brand"><img src="/favicon.svg" width="28" height="28" alt="" /><span>Screen-Pin</span></div>
  </header>

  <section class="shortcuts" aria-labelledby="page-title">
    <h1 id="page-title">Atajos de teclado</h1>
    <p class="shortcut-intro">Úsalos desde cualquier ventana.</p>
    <div class="shortcut-row">
      <div><strong>Nuevo recorte</strong>{#if shortcutWarnings.includes('capture')}<span class="shortcut-warning" role="status">Atajo ocupado · usa el botón</span>{/if}</div>
      <kbd>Alt + Shift + S</kbd>
    </div>
    <div class="shortcut-row">
      <div><strong>Abrir memoria</strong>{#if shortcutWarnings.includes('memory')}<span class="shortcut-warning" role="status">Atajo ocupado · usa el botón o la bandeja</span>{/if}</div>
      <kbd>Alt + Shift + M</kbd>
    </div>
    <div class="shortcut-row">
      <div><strong>Recuperar clics</strong>{#if shortcutWarnings.includes('restore')}<span class="shortcut-warning" role="status">Atajo ocupado · abre Memoria o usa la bandeja</span>{/if}</div>
      <kbd>Alt + Shift + G</kbd>
    </div>
  </section>

  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !desktop}<p class="notice">Vista previa: las capturas y los atajos requieren la aplicación de escritorio.</p>{/if}

  <footer class="companion-footer">
    <button class="button primary" disabled={!desktop || busy} onclick={() => run('trigger_capture')}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M8 3H3V8M16 3H21V8M21 16V21H16M8 21H3V16M8 12H16M12 8V16" /></svg>
      {busy ? 'Un momento…' : 'Nuevo recorte'}
    </button>
    {#if desktop}
      <button class="button memory-link" disabled={busy} onclick={() => run('open_memory_drawer')}>Abrir memoria</button>
    {:else}
      <a class="button memory-link" href="/memory">Ver memoria</a>
    {/if}
  </footer>
</main>

<style>
  .companion-shell { height: 100dvh; padding: 22px 28px; display: flex; flex-direction: column; overflow: hidden; background: var(--chassis-dark); color: var(--chassis-light); }
  .brand-row, .brand, .shortcut-row, .companion-footer { display: flex; align-items: center; }
  .brand-row { padding-bottom: 12px; border-bottom: 1px solid var(--border-color); }
  .brand { gap: 8px; font-size: 14px; font-weight: 650; letter-spacing: -.3px; }
  .shortcuts { padding: 22px 0; }
  h1 { font-size: 23px; line-height: 1.2; font-weight: 600; letter-spacing: -.6px; }
  .shortcut-intro { color: var(--text-secondary); font-size: 12px; margin: 6px 0 16px; }
  .shortcut-row { justify-content: space-between; gap: 16px; min-height: 54px; padding: 10px 0; border-top: 1px solid var(--border-color); }
  .shortcut-row > div { min-width: 0; }
  .shortcut-row strong { display: block; font-size: 13px; font-weight: 500; }
  .shortcut-row kbd { flex-shrink: 0; font-size: 11px; padding: 5px 8px; }
  .shortcut-warning { display: block; margin-top: 4px; color: var(--accent-focus); font-size: 11px; }
  .companion-footer { gap: 10px; margin-top: auto; padding-top: 14px; border-top: 1px solid var(--border-color); }
  .companion-footer .button { flex: 1; min-height: 40px; text-decoration: none; }
  @media (max-width: 480px) { .companion-shell { padding: 18px; } .shortcut-row { gap: 8px; } .shortcut-row kbd { font-size: 10px; } }
</style>
