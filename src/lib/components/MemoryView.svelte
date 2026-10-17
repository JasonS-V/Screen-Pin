<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { assetUrl, errorMessage, isTauri } from '$lib/desktop';
  import type { PinRecord } from '$lib/types';

  const desktop = isTauri();
  let pins = $state<PinRecord[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(true);
  let error = $state('');
  let feedback = $state('');
  let busy = $state('');
  let now = $state(Date.now() / 1000);
  let requestId = 0;
  let searchTimer: ReturnType<typeof setTimeout>;
  let feedbackTimer: ReturnType<typeof setTimeout>;

  onMount(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    const clock = setInterval(() => { now = Date.now() / 1000; }, 30000);
    if (desktop) {
      listen('memory:changed', () => { void loadPins(); }).then(fn => {
        if (disposed) fn(); else unlisten = fn;
      }).catch(cause => { error = errorMessage(cause); });
    }
    void loadPins();
    return () => { disposed = true; requestId++; unlisten?.(); clearInterval(clock); clearTimeout(searchTimer); clearTimeout(feedbackTimer); };
  });

  async function loadPins() {
    const id = ++requestId;
    isLoading = true;
    if (!desktop) { isLoading = false; return; }
    try {
      const query = searchQuery.trim();
      const result = await invoke<PinRecord[]>(query ? 'search_memory_pins' : 'get_memory_pins', query ? { query } : {});
      if (id === requestId) { pins = result; error = ''; }
    } catch (cause) { if (id === requestId) error = errorMessage(cause); }
    finally { if (id === requestId) isLoading = false; }
  }

  function search() {
    requestId++;
    clearTimeout(searchTimer);
    isLoading = true;
    searchTimer = setTimeout(loadPins, 180);
  }

  async function action(command: string, pinId = '') {
    if (busy || !desktop) return;
    busy = pinId || command;
    error = '';
    try { await invoke(command, pinId ? { pinId } : {}); if (command === 'toggle_retention') await loadPins(); }
    catch (cause) { error = errorMessage(cause); }
    finally { busy = ''; }
  }

  async function copyText(pin: PinRecord) {
    try {
      await navigator.clipboard.writeText(pin.raw_text.trim());
      feedback = pin.id;
      clearTimeout(feedbackTimer);
      feedbackTimer = setTimeout(() => { feedback = ''; }, 1600);
    } catch { error = 'No se pudo copiar el texto al portapapeles. Inténtalo de nuevo.'; }
  }

  function age(timestamp: number) {
    const minutes = Math.max(0, Math.floor((now - timestamp) / 60));
    if (minutes === 0) return 'Ahora';
    if (minutes < 60) return `Hace ${minutes} min`;
    if (minutes < 1440) return `Hace ${Math.floor(minutes / 60)} h`;
    return new Date(timestamp * 1000).toLocaleDateString('es', { day: 'numeric', month: 'short' });
  }
  function retention(pin: PinRecord) {
    if (pin.is_pinned_forever) return 'Conservado';
    const minutes = Math.ceil(((pin.expires_at ?? now) - now) / 60);
    return minutes > 0 ? `Se elimina en ${minutes} min` : 'Pendiente de limpieza';
  }
  function label(pin: PinRecord) {
    return pin.entity_type === 'BankDigits' ? 'Datos bancarios' : pin.entity_type === 'WifiCredential' ? 'Credencial' : 'Recorte';
  }
</script>

<svelte:head><title>Memoria · Screen-Pin</title></svelte:head>
<svelte:window onfocus={() => { if (desktop) void loadPins(); }} />

<main class="memory-shell">
  <header class="memory-header">
    <div class="heading-row"><h1>Memoria</h1>
      <button class="button new-button" disabled={!desktop || !!busy} onclick={() => action('trigger_capture')} aria-label="Nuevo recorte">+ <span>Nuevo recorte</span></button></div>
    <div class="search-box">
      <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5" /><path d="M13 13L17.5 17.5" /></svg>
      <input type="search" bind:value={searchQuery} oninput={search} placeholder="Buscar recortes…" aria-label="Buscar en el texto de los recortes" />
    </div>
    <span class="sr-only" role="status">{isLoading ? 'Cargando recortes' : `${pins.length}${pins.length === 50 ? '+' : ''} recorte${pins.length === 1 ? '' : 's'}`}</span>
  </header>

  <section class="pins-feed" aria-label="Recortes guardados" aria-busy={isLoading}>
    {#if error}<div class="notice error" role="alert">{error}<button class="retry" onclick={loadPins}>Reintentar</button></div>{/if}
    {#if !desktop}
      <div class="empty-state"><div class="empty-icon" aria-hidden="true">⌑</div><h2>Tu memoria vive en el escritorio</h2><p>Abre Screen-Pin para consultar tus capturas locales. Tus recortes no se cargan en esta vista previa.</p><a href="/" class="button">Volver al inicio</a></div>
    {:else if isLoading && pins.length === 0}
      <div class="loading-state" role="status">Cargando tus recortes…</div>
    {:else if !error && pins.length === 0}
      <div class="empty-state"><h2>{searchQuery.trim() ? 'Sin coincidencias' : 'Aún no hay recortes'}</h2>
        <p>{searchQuery.trim() ? 'Prueba con otra palabra.' : 'Crea un recorte para verlo aquí.'}</p>
        {#if searchQuery.trim()}<button class="button" onclick={() => { searchQuery = ''; void loadPins(); }}>Limpiar búsqueda</button>
        {:else}<button class="button primary" disabled={!!busy} onclick={() => action('trigger_capture')}>Crear mi primer recorte</button>{/if}
      </div>
    {:else}
      <ul class="cards-list">
        {#each pins as pin (pin.id)}
          <li class="memory-card">
            <button class="preview" onclick={() => action('repin_from_memory', pin.id)} disabled={!!busy} aria-label="Volver a fijar este recorte">
              <img src={assetUrl(pin.id)} alt="" loading="lazy" /><span class="preview-label">Fijar ↗</span>
            </button>
            <div class="card-content">
              <div class="card-meta"><span class="type-label">{label(pin)}</span><time datetime={new Date(pin.created_at * 1000).toISOString()} title={new Date(pin.created_at * 1000).toLocaleString('es')}>{age(pin.created_at)}</time></div>
              {#if pin.raw_text.trim()}<p class="snippet" title={pin.raw_text}>{pin.raw_text.trim()}</p>
              {:else if pin.ocr_status === 'pending'}<p class="ocr-note">Reconociendo texto…</p>
              {:else if pin.ocr_status === 'error'}<p class="ocr-note">Lectura de texto no disponible</p>{/if}
              {#if pin.is_sensitive}<div class="retention"><span aria-hidden="true">◷</span><span>{retention(pin)}</span></div>{/if}
              {#if pin.raw_text.trim() || pin.is_sensitive}<div class="card-actions">
                {#if pin.raw_text.trim()}<button class="card-btn" onclick={() => copyText(pin)}>{feedback === pin.id ? 'Copiado ✓' : 'Copiar texto'}</button>{/if}
                {#if pin.is_sensitive}<button class="card-btn retain" aria-pressed={pin.is_pinned_forever} disabled={!!busy} onclick={() => action('toggle_retention', pin.id)} title={pin.is_pinned_forever ? 'Volver a eliminar automáticamente en 30 minutos' : 'Conservar sin caducidad'}>{pin.is_pinned_forever ? 'Liberar' : 'Conservar'}</button>{/if}
              </div>{/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  <span class="sr-only" role="status">{feedback ? 'Texto copiado al portapapeles' : ''}</span>
</main>

<style>
  .memory-shell { display: flex; flex-direction: column; height: 100dvh; background: var(--chassis-dark); color: var(--chassis-light); }
  .memory-header { padding: 16px 16px 0; }
  .heading-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  h1 { font-weight: 600; letter-spacing: -.6px; font-size: 22px; }
  .new-button { font-size: 15px; padding: 5px 10px; } .new-button span { font-size: 11px; }
  .search-box { display: flex; align-items: center; gap: 8px; margin: 12px 0 0; padding: 8px 10px; background: #1e1e1c; border: 1px solid var(--border-strong); border-radius: 3px; color: var(--text-secondary); }
  .search-box:focus-within { outline: 2px solid var(--accent-focus); outline-offset: 2px; }
  .search-box input { border: 0; background: transparent; color: var(--chassis-light); font-size: 12px; width: 100%; min-width: 0; outline: none; }
  input::placeholder { color: var(--text-secondary); }
  .search-box svg { flex-shrink: 0; }
  .pins-feed { flex: 1; min-height: 0; overflow-y: auto; padding: 12px 16px; scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent; }
  .cards-list { list-style: none; display: grid; gap: 8px; }
  .memory-card { min-width: 0; display: grid; grid-template-columns: 84px minmax(0, 1fr); gap: 10px; background: #1c1c1a; border: 1px solid var(--border-color); border-radius: 4px; padding: 8px; }
  .card-content { min-width: 0; }
  .card-meta { display: flex; justify-content: space-between; gap: 8px; font-size: 10px; color: var(--text-secondary); }
  .type-label { color: var(--chassis-light); }
  .preview { position: relative; display: block; width: 84px; height: 84px; overflow: hidden; background: var(--surface-secondary); border: 1px solid var(--border-color); border-radius: 2px; cursor: pointer; }
  .preview img { display: block; height: 100%; width: 100%; object-fit: contain; }
  .preview-label { position: absolute; bottom: 0; left: 0; right: 0; font-size: 10px; padding: 4px; color: var(--chassis-light); background: rgba(20, 20, 19, .9); }
  .snippet { margin: 6px 0 0; font: 11px/1.4 var(--font-mono); overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow-wrap: anywhere; white-space: pre-line; user-select: text; }
  .ocr-note { font-size: 11px; color: var(--text-secondary); margin-top: 6px; }
  .retention { display: flex; gap: 6px; align-items: center; color: var(--accent-focus); font-size: 10px; margin-top: 6px; }
  .card-actions { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
  .card-btn { background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11px; padding: 4px 6px; cursor: pointer; border-radius: 2px; }
  .card-btn:hover { background: var(--surface-secondary); color: var(--chassis-light); } .retain { color: var(--accent-focus); }
  .empty-state { min-height: 180px; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 10px; padding: 16px 8px; max-width: 360px; margin: auto; }
  .empty-icon { font-size: 42px; color: var(--accent-focus); line-height: 1; } .empty-state h2 { font-size: 17px; font-weight: 500; } .empty-state p { font-size: 12px; line-height: 1.7; color: var(--text-secondary); }
  .empty-state a { text-decoration: none; } .loading-state { padding: 48px 0; text-align: center; color: var(--text-secondary); font-size: 12px; }
  .retry { display: block; margin-top: 6px; background: transparent; border: 0; color: var(--accent-focus); font-size: 12px; text-decoration: underline; cursor: pointer; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  @media (max-width: 360px) { .new-button span { display: none; } }
</style>
