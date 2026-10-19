<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { assetUrl, errorMessage, isTauri } from '$lib/desktop';
  import type { PinRecord } from '$lib/types';
  import { startDrag } from '@crabnebula/tauri-plugin-drag';
  import { i18n } from '$lib/i18n.svelte';

  const desktop = isTauri();
  let pins = $state<PinRecord[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(true);
  let error = $state('');
  let feedback = $state('');
  let imageFeedback = $state('');
  let busy = $state('');
  let now = $state(Date.now() / 1000);
  let requestId = 0;
  let searchTimer: ReturnType<typeof setTimeout>;
  let feedbackTimer: ReturnType<typeof setTimeout>;
  let imageFeedbackTimer: ReturnType<typeof setTimeout>;

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
    return () => { disposed = true; requestId++; unlisten?.(); clearInterval(clock); clearTimeout(searchTimer); clearTimeout(feedbackTimer); clearTimeout(imageFeedbackTimer); };
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
    } catch { error = i18n.t.memoryCopyTextError; }
  }

  async function copyImage(pin: PinRecord) {
    if (!desktop) return;
    try {
      await invoke('copy_pin_image', { pinId: pin.id });
      imageFeedback = pin.id;
      clearTimeout(imageFeedbackTimer);
      imageFeedbackTimer = setTimeout(() => { imageFeedback = ''; }, 1600);
    } catch {
      error = i18n.t.copyImageError;
    }
  }

  let lastDragTimestamp = 0;

  async function handlePreviewPointerDown(e: PointerEvent, pin: PinRecord) {
    if (e.button !== 0 || !desktop) return;
    const startX = e.clientX;
    const startY = e.clientY;
    let dragStarted = false;

    const onMove = async (me: PointerEvent) => {
      if (dragStarted) return;
      if (Math.hypot(me.clientX - startX, me.clientY - startY) >= 6) {
        dragStarted = true;
        lastDragTimestamp = Date.now();
        window.removeEventListener('pointermove', onMove);
        try {
          const path = await invoke<string>('get_pin_image_path', { pinId: pin.id });
          if (path) {
            await startDrag({
              item: [path],
              icon: path,
              mode: 'copy'
            });
          }
        } catch (err) {
          console.warn('Error arrastrando desde memoria:', err);
        }
      }
    };

    const onUp = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };

    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  }

  function age(timestamp: number) {
    const minutes = Math.max(0, Math.floor((now - timestamp) / 60));
    if (minutes === 0) return i18n.t.timeNow;
    if (minutes < 60) return i18n.t.timeMinutesAgo(minutes);
    if (minutes < 1440) return i18n.t.timeHoursAgo(Math.floor(minutes / 60));
    return new Date(timestamp * 1000).toLocaleDateString(i18n.current, { day: 'numeric', month: 'short' });
  }
  function retention(pin: PinRecord) {
    if (pin.is_pinned_forever) return i18n.t.keptForever;
    const minutes = Math.ceil(((pin.expires_at ?? now) - now) / 60);
    return minutes > 0 ? i18n.t.expiresInMin(minutes) : i18n.t.pendingCleanup;
  }
  function label(pin: PinRecord) {
    return pin.entity_type === 'BankDigits' ? i18n.t.bankDigitsBadge : pin.entity_type === 'WifiCredential' ? i18n.t.wifiCredentialBadge : i18n.t.genericPinBadge;
  }
</script>

<svelte:head><title>{i18n.t.memoryTitle}</title></svelte:head>
<svelte:window onfocus={() => { if (desktop) void loadPins(); }} />

<main class="memory-shell">
  <header class="memory-header">
    <div class="heading-row">
      <div class="heading-title-group">
        <h1>{i18n.t.memoryHeading}</h1>
        <div class="lang-switch-pill" role="group" aria-label={i18n.t.languageLabel}>
          <button
            type="button"
            class="lang-pill-btn"
            class:active={i18n.current === 'en'}
            title="Switch to English"
            onclick={() => i18n.set('en')}
          >EN</button>
          <span class="lang-divider">/</span>
          <button
            type="button"
            class="lang-pill-btn"
            class:active={i18n.current === 'es'}
            title="Cambiar a Español"
            onclick={() => i18n.set('es')}
          >ES</button>
        </div>
      </div>
      <button class="button new-button" disabled={!desktop || !!busy} onclick={() => action('trigger_capture')} aria-label={i18n.t.newCrop}>+ <span>{i18n.t.newCrop}</span></button>
    </div>
    <div class="search-box">
      <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5" /><path d="M13 13L17.5 17.5" /></svg>
      <input type="search" bind:value={searchQuery} oninput={search} placeholder={i18n.t.searchPlaceholder} aria-label={i18n.t.searchAria} />
    </div>
    <span class="sr-only" role="status">{isLoading ? i18n.t.loadingPins : i18n.t.pinCount(pins.length, pins.length === 50)}</span>
  </header>

  <section class="pins-feed" aria-label={i18n.t.savedPinsAria} aria-busy={isLoading}>
    {#if error}<div class="notice error" role="alert">{error}<button class="retry" onclick={loadPins}>{i18n.t.retry}</button></div>{/if}
    {#if !desktop}
      <div class="empty-state">
        <div class="empty-icon" aria-hidden="true">
          <svg width="42" height="42" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>
        </div>
        <h2>{i18n.t.memoryDesktopTitle}</h2>
        <p>{i18n.t.memoryDesktopDesc}</p>
        <a href="/" class="button">{i18n.t.backToHome}</a>
      </div>
    {:else if isLoading && pins.length === 0}
      <div class="loading-state" role="status">{i18n.t.loadingYourPins}</div>
    {:else if !error && pins.length === 0}
      <div class="empty-state"><h2>{searchQuery.trim() ? i18n.t.noMatches : i18n.t.noPinsYet}</h2>
        <p>{searchQuery.trim() ? i18n.t.tryAnotherKeyword : i18n.t.createPinPrompt}</p>
        {#if searchQuery.trim()}<button class="button" onclick={() => { searchQuery = ''; void loadPins(); }}>{i18n.t.clearSearch}</button>
        {:else}<button class="button primary" disabled={!!busy} onclick={() => action('trigger_capture')}>{i18n.t.createFirstPin}</button>{/if}
      </div>
    {:else}
      <ul class="cards-list">
        {#each pins as pin (pin.id)}
          <li class="memory-card">
            <button
              class="preview"
              onclick={() => {
                if (Date.now() - lastDragTimestamp < 400) return;
                action('repin_from_memory', pin.id);
              }}
              onpointerdown={(e) => handlePreviewPointerDown(e, pin)}
              disabled={!!busy}
              title={i18n.t.repinTitle}
              aria-label={i18n.t.repinAria}
            >
              <img src={assetUrl(pin.id)} alt="" loading="lazy" draggable="false" />
              <span class="preview-label">
                {i18n.t.repinBadge}
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="7" y1="17" x2="17" y2="7"/><polyline points="7 7 17 7 17 17"/></svg>
              </span>
            </button>
            <div class="card-content">
              <div class="card-meta"><span class="type-label">{label(pin)}</span><time datetime={new Date(pin.created_at * 1000).toISOString()} title={new Date(pin.created_at * 1000).toLocaleString(i18n.current)}>{age(pin.created_at)}</time></div>
              {#if pin.raw_text.trim()}<p class="snippet" title={pin.raw_text}>{pin.raw_text.trim()}</p>
              {:else if pin.ocr_status === 'pending'}<p class="ocr-note">{i18n.t.ocrTextLoadingNote}</p>
              {:else if pin.ocr_status === 'error'}<p class="ocr-note">{i18n.t.ocrTextErrorNote}</p>{/if}
              {#if pin.is_sensitive}
                <div class="retention">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
                  <span>{retention(pin)}</span>
                </div>
              {/if}
              <div class="card-actions">
                <button class="card-btn" onclick={() => copyImage(pin)}>
                  {#if imageFeedback === pin.id}
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
                    <span>{i18n.t.imageCopied}</span>
                  {:else}
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                    <span>{i18n.t.copyImage}</span>
                  {/if}
                </button>
                {#if pin.raw_text.trim()}
                  <button class="card-btn" onclick={() => copyText(pin)}>
                    {#if feedback === pin.id}
                      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
                      <span>{i18n.t.textCopied}</span>
                    {:else}
                      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
                      <span>{i18n.t.copyText}</span>
                    {/if}
                  </button>
                {/if}
                {#if pin.is_sensitive}<button class="card-btn retain" aria-pressed={pin.is_pinned_forever} disabled={!!busy} onclick={() => action('toggle_retention', pin.id)} title={pin.is_pinned_forever ? i18n.t.releaseTitle : i18n.t.keepTitle}>{pin.is_pinned_forever ? i18n.t.releaseBtn : i18n.t.keepBtn}</button>{/if}
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  <span class="sr-only" role="status">{feedback ? i18n.t.textCopied : ''}</span>
</main>

<style>
  .memory-shell { display: flex; flex-direction: column; height: 100dvh; background: var(--chassis-dark); color: var(--chassis-light); }
  .memory-header { padding: 16px 16px 0; }
  .heading-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .heading-title-group { display: flex; align-items: center; gap: 10px; }
  .lang-switch-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px 4px;
    background: #1b1b19;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
  }
  .lang-pill-btn {
    background: transparent;
    border: none;
    padding: 2px 5px;
    font-size: 10px;
    font-weight: 600;
    font-family: inherit;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 3px;
    transition: all 120ms ease;
  }
  .lang-pill-btn:hover {
    color: var(--chassis-light);
  }
  .lang-pill-btn.active {
    background: var(--surface-secondary);
    color: var(--accent-focus, #FF8A50);
  }
  .lang-divider {
    color: #444440;
    font-size: 10px;
    padding: 0 1px;
    user-select: none;
  }
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
  .card-btn { background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11px; padding: 4px 6px; cursor: pointer; border-radius: 2px; display: inline-flex; align-items: center; gap: 4px; }
  .card-btn:hover { background: var(--surface-secondary); color: var(--chassis-light); } .retain { color: var(--accent-focus); }
  .empty-state { min-height: 180px; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 10px; padding: 16px 8px; max-width: 360px; margin: auto; }
  .empty-icon { display: flex; align-items: center; justify-content: center; color: var(--accent-focus); line-height: 1; } .empty-state h2 { font-size: 17px; font-weight: 500; } .empty-state p { font-size: 12px; line-height: 1.7; color: var(--text-secondary); }
  .empty-state a { text-decoration: none; } .loading-state { padding: 48px 0; text-align: center; color: var(--text-secondary); font-size: 12px; }
  .retry { display: block; margin-top: 6px; background: transparent; border: 0; color: var(--accent-focus); font-size: 12px; text-decoration: underline; cursor: pointer; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  @media (max-width: 360px) { .new-button span { display: none; } }
</style>
