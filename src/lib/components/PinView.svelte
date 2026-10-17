<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { assetUrl, errorMessage, isTauri } from '$lib/desktop';
  import type { OcrLine, OcrPayload, PinRecord, SmartEntity, TargetWindowInfo } from '$lib/types';
  import { playClickSound } from '$lib/sound';

  let { pinId, toolbar = false }: { pinId: string; toolbar?: boolean } = $props();
  let lines = $state<OcrLine[]>([]);
  let entities = $state<SmartEntity[]>([]);
  let fullText = $state('');
  let ocrStatus = $state<'loading' | 'ready' | 'empty' | 'error'>('loading');
  let isGhostMode = $state(false);
  let copyTextFeedback = $state(false);
  let copyImageFeedback = $state(false);
  let copyFeedback = $state('');
  let error = $state('');
  let busy = $state(false);
  let viewHeight = $state(0);
  let feedbackTimer: ReturnType<typeof setTimeout>;
  let imageFeedbackTimer: ReturnType<typeof setTimeout>;
  const bankEntity = $derived(entities.find(e => e.kind === 'BankDigits'));

  // Estado para detección y pegado inteligente en aplicaciones externas
  let targetWindow = $state<TargetWindowInfo | null>(null);
  let isPasting = $state(false);

  // Estados para visibilidad animada de la barra de opciones
  let isNear = $state(false);
  let isHovered = $state(false);
  let isVisible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout>;
  let interactiveTimer: ReturnType<typeof setTimeout>;

  function updateVisibility(show: boolean) {
    clearTimeout(hideTimer);
    clearTimeout(interactiveTimer);
    if (show) {
      isVisible = true;
      if (isTauri()) {
        void invoke('set_toolbar_interactive', { pinId, interactive: true });
      }
    } else {
      hideTimer = setTimeout(() => {
        isVisible = false;
        interactiveTimer = setTimeout(() => {
          if (!isVisible && isTauri()) {
            void invoke('set_toolbar_interactive', { pinId, interactive: false });
          }
        }, 260);
      }, 280);
    }
  }

  $effect(() => {
    if (!toolbar) return;
    const shouldBeVisible = isNear || isHovered;
    updateVisibility(shouldBeVisible);
  });

  function applyPayload(payload: OcrPayload) {
    lines = payload.lines;
    entities = payload.entities;
    fullText = payload.full_text;
    ocrStatus = fullText.trim() ? 'ready' : 'empty';
  }

  onMount(() => {
    if (!isTauri()) {
      if (!toolbar) {
        ocrStatus = 'error';
        error = 'Abre este recorte en Screen-Pin para ver su imagen y texto.';
      } else {
        isVisible = true;
      }
      return;
    }
    let disposed = false;
    const listeners: UnlistenFn[] = [];
    async function initialize() {
      try {
        for (const [name, handler] of [
          [`ocr:ready:${pinId}`, (event: { payload: OcrPayload }) => applyPayload(event.payload)],
          [`ocr:error:${pinId}`, () => { ocrStatus = 'error'; }],
          ['ghost:changed', (event: { payload: boolean }) => { isGhostMode = event.payload; }],
          ['pin:target-window', (event: { payload: TargetWindowInfo | null }) => { targetWindow = event.payload; }],
          ['pin:trigger-paste-anim', (event: { payload: TargetWindowInfo | null }) => {
            if (event.payload) targetWindow = event.payload;
            isPasting = true;
            playClickSound();
          }],
        ] as const) {
          const unlisten = await listen(name, handler as (event: { payload: unknown }) => void, { target: `${toolbar ? 'pin-toolbar' : 'pin'}-${pinId}` });
          if (disposed) { unlisten(); return; }
          listeners.push(unlisten);
        }

        if (toolbar) {
          const unlistenProximity = await listen<boolean>('toolbar:proximity', (event) => {
            isNear = event.payload;
          });
          if (disposed) { unlistenProximity(); return; }
          listeners.push(unlistenProximity);

          // Inicialmente la barra está oculta y no intercepta clics
          void invoke('set_toolbar_interactive', { pinId, interactive: false });
        }

        // Consultar ventana destino inicial si existe bajo el recorte
        invoke<TargetWindowInfo | null>('get_target_window_under_pin', { pinId })
          .then(tw => { if (!disposed && tw) targetWindow = tw; })
          .catch(() => {});

        const snapshot = await invoke<{ pin: PinRecord; entities: SmartEntity[] }>('get_pin', { pinId });
        if (disposed || ocrStatus !== 'loading') return;
        const pin = snapshot.pin;
        if (pin.ocr_status === 'error') { ocrStatus = 'error'; return; }
        if (pin.ocr_status === 'pending') return;
        applyPayload({ pin_id: pinId, lines: pin.ocr_json ? JSON.parse(pin.ocr_json) : [], full_text: pin.raw_text, entities: snapshot.entities, is_sensitive: pin.is_sensitive });
      } catch (cause) { if (!disposed) { ocrStatus = 'error'; error = errorMessage(cause); } }
    }
    void initialize();
    return () => {
      disposed = true;
      listeners.forEach(fn => fn());
      clearTimeout(feedbackTimer);
      clearTimeout(imageFeedbackTimer);
      clearTimeout(hideTimer);
      clearTimeout(interactiveTimer);
    };
  });

  async function archivePin() {
    if (busy) return;
    busy = true;
    error = '';
    playClickSound();
    try { await invoke('archive_pin_to_memory', { pinId }); }
    catch (cause) { error = errorMessage(cause); }
    finally { busy = false; }
  }

  function handleBackgroundDoubleClick(event: MouseEvent) {
    if ((event.target as HTMLElement).closest('.ocr-word, button') || window.getSelection()?.toString()) return;
    void archivePin();
  }

  async function copyPinImageToClipboard() {
    try {
      playClickSound();
      let success = false;
      if (isTauri()) {
        try {
          await invoke('copy_pin_image', { pinId });
          success = true;
        } catch (cause) {
          console.warn('Rust copy_pin_image fallo, probando navegador:', cause);
        }
      }
      if (!success) {
        const response = await fetch(assetUrl(pinId));
        const blob = await response.blob();
        await navigator.clipboard.write([
          new ClipboardItem({ 'image/png': blob })
        ]);
      }
      copyImageFeedback = true;
      clearTimeout(imageFeedbackTimer);
      imageFeedbackTimer = setTimeout(() => { copyImageFeedback = false; }, 1600);
    } catch {
      error = 'No se pudo copiar la imagen al portapapeles.';
    }
  }

  async function copyAllText() {
    if (!fullText.trim()) return;
    try {
      playClickSound();
      await navigator.clipboard.writeText(fullText.trim());
      copyTextFeedback = true;
      copyFeedback = 'Texto copiado';
      clearTimeout(feedbackTimer);
      feedbackTimer = setTimeout(() => { copyTextFeedback = false; copyFeedback = ''; }, 1600);
    } catch {
      error = 'No se pudo copiar el texto al portapapeles.';
    }
  }

  function handleCopy(event: ClipboardEvent) {
    const text = window.getSelection()?.toString();
    if (text) {
      if (event.clipboardData) {
        event.clipboardData.setData('text/plain', text);
        event.preventDefault();
      }
      return;
    }
    // Si no hay texto seleccionado, copiar la imagen al portapapeles con Ctrl+C
    void copyPinImageToClipboard();
  }

  async function copyQuickValue(text: string, label: string) {
    try {
      playClickSound();
      await navigator.clipboard.writeText(text);
      copyFeedback = `${label} copiado`;
      copyTextFeedback = true;
      clearTimeout(feedbackTimer);
      feedbackTimer = setTimeout(() => { copyTextFeedback = false; copyFeedback = ''; }, 1600);
    } catch { error = 'No se pudo copiar al portapapeles.'; }
  }

  async function toggleGhostMode() {
    playClickSound();
    try {
      await invoke('toggle_ghost_mode', { pinId, enabled: !isGhostMode });
    } catch (cause) { error = errorMessage(cause); }
  }

  function handlePinMouseEnter() {
    if (isTauri()) {
      void emitTo(`pin-toolbar-${pinId}`, 'toolbar:proximity', true);
    }
  }

  function handlePinMouseLeave() {
    if (isTauri()) {
      void emitTo(`pin-toolbar-${pinId}`, 'toolbar:proximity', false);
    }
  }

  async function pasteToTarget() {
    if (busy || isPasting) return;
    if (!targetWindow || !targetWindow.is_valid) return;
    isPasting = true;
    error = '';
    playClickSound();

    if (toolbar && isTauri()) {
      void emitTo(`pin-${pinId}`, 'pin:trigger-paste-anim', targetWindow);
    }

    try {
      if (isTauri()) {
        await invoke('paste_pin_to_target', {
          pinId,
          targetHwnd: targetWindow.hwnd
        });
      }
      setTimeout(() => {
        void archivePin();
      }, 340);
    } catch (cause) {
      isPasting = false;
      error = errorMessage(cause);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.repeat) return;
    if (event.key === 'Escape') { event.preventDefault(); void archivePin(); }
    else if ((event.target as HTMLElement).closest('button, input, textarea')) return;
    else if (event.altKey && event.key.toLowerCase() === 'g') { event.preventDefault(); void toggleGhostMode(); }
    else if (event.key === 'Enter' && bankEntity?.kind === 'BankDigits') { event.preventDefault(); void copyQuickValue(bankEntity.value.clean_digits, 'Número'); }
    else if ((event.ctrlKey && event.key.toLowerCase() === 'v') || (event.key === 'Enter' && targetWindow?.is_valid)) {
      event.preventDefault();
      void pasteToTarget();
    }
  }
</script>

<svelte:head><title>Recorte · Screen-Pin</title></svelte:head>
<svelte:window onkeydown={handleKeydown} oncopy={handleCopy} bind:innerHeight={viewHeight} />

{#if toolbar}
<div
  class="pin-toolbar-viewport"
  role="presentation"
  onmouseenter={() => { isHovered = true; }}
  onmouseleave={() => { isHovered = false; }}
>
  <main
    class="pin-toolbar"
    class:is-visible={isVisible}
    class:has-error={!!error}
    title={error || undefined}
    aria-label="Acciones del recorte"
  >
    <button
      class="toolbar-btn"
      title="Copiar imagen al portapapeles para pegar en cualquier lugar (Ctrl+V)"
      onclick={copyPinImageToClipboard}
    >
      {#if copyImageFeedback}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
        <span>Imagen copiada</span>
      {:else}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        <span>Copiar imagen</span>
      {/if}
    </button>
    {#if targetWindow && targetWindow.is_valid}
      <button
        class="toolbar-btn paste-btn"
        title="Pegar automáticamente en {targetWindow.app_name} (Ctrl+V)"
        onclick={pasteToTarget}
        disabled={isPasting}
      >
        {#if isPasting}
          <span>Pegando…</span>
        {:else}
          <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg>
          <span>Pegar en {targetWindow.app_name}</span>
        {/if}
      </button>
    {/if}
    <button
      class="toolbar-btn"
      disabled={!fullText.trim()}
      title={ocrStatus === 'loading' ? 'Reconociendo texto…' : ocrStatus === 'error' ? 'Lectura no disponible' : ocrStatus === 'empty' ? 'No se detectó texto' : 'Copiar todo el texto'}
      onclick={copyAllText}
    >
      {#if copyTextFeedback}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
        <span>Texto copiado</span>
      {:else}
        <svg class="btn-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
        <span>Copiar texto</span>
      {/if}
    </button>
    <button
      class="toolbar-btn"
      class:active={isGhostMode}
      onclick={toggleGhostMode}
      title="Dejar pasar los clics. Recuperar con Alt+Shift+G o desde la bandeja."
      aria-pressed={isGhostMode}
    >
      Fantasma
    </button>
    <button
      class="toolbar-btn"
      onclick={archivePin}
      disabled={busy}
      title="Guardar en Memoria y cerrar"
    >
      Archivar
    </button>
    {#if error}<span class="sr-only" role="alert">{error}</span>{/if}
  </main>
</div>
{:else}
<main
  class="pin-chassis"
  class:is-ghost={isGhostMode}
  class:is-pasting={isPasting}
  ondblclick={handleBackgroundDoubleClick}
  onmouseenter={handlePinMouseEnter}
  onmouseleave={handlePinMouseLeave}
  aria-label="Recorte de pantalla flotante"
>
  <div
    class="calibration-corner"
    class:is-scanning={ocrStatus === 'loading'}
    data-tauri-drag-region
    title="Arrastrar para mover ventana"
  ></div>

  <div
    class="window-drag-bar"
    data-tauri-drag-region
    title="Arrastrar para mover ventana"
  ></div>

  <div
    class="image-stage"
    data-tauri-drag-region
    role="region"
    aria-label="Captura fijada en pantalla"
    title="Arrastra para mover la ventana sobre otra aplicación para pegarla"
  >
    <img
      src={isTauri() ? assetUrl(pinId) : undefined}
      alt="Captura fijada en pantalla"
      draggable="false"
      onerror={() => { error = 'La imagen del recorte ya no está disponible.'; }}
    />

    {#if ocrStatus === 'ready'}
      <div class="ocr-layer" aria-label="Texto extraído de la imagen">
        {#each lines as line, lineIndex}
          {#each line.words as word, wordIndex}
            <span
              class="ocr-word"
              style:left="{word.bounds.x * 100}%"
              style:top="{word.bounds.y * 100}%"
              style:width="{word.bounds.width * 100}%"
              style:height="{word.bounds.height * 100}%"
              style:font-size="{Math.max(1, word.bounds.height * viewHeight * 0.85)}px"
            >
              {word.text}{wordIndex < line.words.length - 1 ? ' ' : lineIndex < lines.length - 1 ? '\n' : ''}
            </span>
          {/each}
        {/each}
      </div>
    {/if}
  </div>

  {#if targetWindow && targetWindow.is_valid && !isPasting}
    <div class="pin-paste-bar">
      <button
        type="button"
        class="pin-paste-pill"
        onclick={pasteToTarget}
        title="Pegar recorte directamente en {targetWindow.app_name} (Ctrl+V)"
      >
        <svg class="pill-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>
          <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
        </svg>
        <span>Pegar en <strong>{targetWindow.app_name}</strong></span>
      </button>
    </div>
  {/if}

  {#if isPasting}
    <div class="pasting-overlay" role="status">
      <svg class="pasting-icon" width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <polyline points="20 6 9 17 4 12"/>
      </svg>
      <span>¡Pegado en {targetWindow?.app_name || 'destino'}!</span>
    </div>
  {/if}

  {#if error}<span class="pin-error" role="alert">{error}</span>{/if}
  {#if copyFeedback}<span class="sr-only" role="status">{copyFeedback}</span>{/if}
</main>
{/if}

<style>
  :global(:root) {
    --chassis-dark: #141413;
    --chassis-light: #F4F3EF;
    --accent-amber: #D95D1E;
    --accent-focus: #FF8A50;
  }

  .pin-chassis {
    position: relative;
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    border-radius: 2px;
    border: 1px solid var(--chassis-light);
    outline: 1px solid var(--chassis-dark);
    outline-offset: -2px;
    overflow: hidden;
    user-select: none;
    transition: opacity 80ms cubic-bezier(0.2, 0, 0, 1);
    background: transparent;
  }

  .pin-chassis:focus-within {
    border-color: var(--accent-focus);
    outline: 2px solid var(--accent-focus);
  }

  .pin-chassis.is-ghost {
    opacity: 0.22;
  }

  .pin-chassis.is-pasting {
    pointer-events: none;
    animation: paste-woosh 340ms cubic-bezier(0.2, 0.9, 0.3, 1) forwards;
  }

  @keyframes paste-woosh {
    0% {
      transform: scale(1) translateY(0);
      opacity: 1;
      box-shadow: 0 0 0 2px var(--accent-focus, #FF8A50), 0 0 20px rgba(255, 138, 80, 0.6);
      filter: brightness(1);
    }
    35% {
      transform: scale(1.04) translateY(-4px);
      box-shadow: 0 0 0 4px #FF8A50, 0 8px 30px rgba(255, 138, 80, 0.85);
      filter: brightness(1.25);
    }
    100% {
      transform: scale(0.35) translateY(45px);
      opacity: 0;
      box-shadow: 0 0 0 1px #FF8A50, 0 0 8px rgba(255, 138, 80, 0.2);
      filter: brightness(1.4);
    }
  }

  .calibration-corner {
    position: absolute;
    top: 0;
    left: 0;
    width: 12px;
    height: 12px;
    border-top: 3px solid var(--accent-amber);
    border-left: 3px solid var(--accent-amber);
    z-index: 30;
    cursor: move;
  }

  .window-drag-bar {
    position: absolute;
    top: 0;
    left: 14px;
    right: 0;
    height: 8px;
    z-index: 29;
    cursor: move;
  }

  .calibration-corner.is-scanning {
    animation: pulse-amber 600ms infinite alternate;
  }

  @keyframes pulse-amber {
    from { border-color: var(--accent-amber); }
    to { border-color: var(--chassis-light); }
  }

  .image-stage {
    position: relative;
    width: 100%;
    height: 100%;
    cursor: move;
  }

  .image-stage:active {
    cursor: grabbing;
  }

  .image-stage img {
    width: 100%;
    height: 100%;
    object-fit: fill;
    display: block;
    pointer-events: none;
  }

  .ocr-layer {
    pointer-events: none;
    position: absolute;
    inset: 0;
    user-select: text;
  }

  .ocr-word {
    pointer-events: auto;
    position: absolute;
    color: transparent;
    font-family: 'JetBrains Mono', monospace;
    line-height: 1;
    white-space: pre;
    cursor: text;
    border-bottom: 1px solid transparent;
    transition: border-color 100ms ease;
  }

  .pin-chassis:hover .ocr-word,
  .pin-chassis:focus-within .ocr-word {
    border-bottom-color: rgba(217, 93, 30, 0.55);
  }

  .ocr-word::selection {
    background-color: rgba(255, 138, 80, 0.3);
    color: transparent;
  }

  .pin-error { position: absolute; bottom: 4px; left: 4px; right: 4px; z-index: 25; padding: 5px 7px; color: var(--accent-focus); background: var(--chassis-dark); font-size: 11px; overflow-wrap: anywhere; }

  .pin-toolbar-viewport {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 4px 14px 26px 14px;
    box-sizing: border-box;
    overflow: visible;
    background: transparent;
  }

  .pin-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    max-width: 520px;
    height: 38px;
    padding: 3px 4px;
    box-sizing: border-box;
    overflow: visible;
    color: var(--chassis-light);
    background: #191918;
    border: 1px solid #383835;
    border-radius: 6px;
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.08),
      0 6px 16px rgba(0, 0, 0, 0.12),
      0 12px 24px rgba(0, 0, 0, 0.14);

    /* Oculto por defecto y desplazado hacia arriba */
    opacity: 0;
    transform: translateY(-10px);
    pointer-events: none;

    /* Animación de retorno */
    transition:
      transform 220ms cubic-bezier(0.4, 0, 0.2, 1),
      opacity 180ms ease-out;
    will-change: transform, opacity;
  }

  .pin-toolbar.is-visible {
    /* Desplazamiento fluido hacia abajo al aparecer */
    opacity: 1;
    transform: translateY(0);
    pointer-events: auto;
    transition:
      transform 280ms cubic-bezier(0.16, 1, 0.3, 1),
      opacity 220ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .pin-toolbar.has-error {
    border-color: var(--accent-focus);
  }

  .toolbar-btn {
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 0 6px;
    border: 0;
    border-radius: 3px;
    background: var(--surface-secondary, #262624);
    color: var(--chassis-light);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    transition: background 120ms ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }

  .btn-icon {
    flex-shrink: 0;
  }

  .toolbar-btn:hover:not(:disabled) {
    background: #393936;
  }

  .toolbar-btn.active {
    background: var(--accent-focus);
    color: var(--chassis-dark);
  }

  .toolbar-btn.paste-btn {
    border: 1px solid rgba(255, 138, 80, 0.45);
    color: #FFB380;
    background: rgba(255, 138, 80, 0.12);
  }

  .toolbar-btn.paste-btn:hover:not(:disabled) {
    background: var(--accent-focus, #FF8A50);
    color: #141413;
    border-color: var(--accent-focus, #FF8A50);
  }

  .pin-paste-bar {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 35;
    pointer-events: auto;
    animation: pill-appear 220ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes pill-appear {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }

  .pin-paste-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    background: rgba(20, 20, 19, 0.92);
    border: 1px solid var(--accent-focus, #FF8A50);
    border-radius: 20px;
    color: #F4F3EF;
    font-size: 11px;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.5), 0 0 10px rgba(255, 138, 80, 0.3);
    transition: all 120ms ease;
    white-space: nowrap;
    user-select: none;
  }

  .pin-paste-pill:hover {
    background: var(--accent-focus, #FF8A50);
    color: #141413;
    box-shadow: 0 4px 16px rgba(255, 138, 80, 0.5);
    transform: translateY(-1px) scale(1.02);
  }

  .pin-paste-pill:hover .pill-icon {
    stroke: #141413;
  }

  .pin-paste-pill strong {
    color: var(--accent-focus, #FF8A50);
    font-weight: 600;
  }

  .pin-paste-pill:hover strong {
    color: #141413;
  }

  .pasting-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: rgba(20, 20, 19, 0.82);
    backdrop-filter: blur(4px);
    color: #F4F3EF;
    font-size: 13px;
    font-weight: 600;
    z-index: 50;
    border-radius: 2px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
