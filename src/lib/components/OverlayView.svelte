<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { MonitorInfo, CropSelection } from '$lib/types';
  import { assetUrl, errorMessage, isTauri } from '$lib/desktop';

  let { monitorId = 0 }: { monitorId?: number } = $props();

  let monitor = $state<MonitorInfo | null>(null);
  let imgLoaded = $state(false);
  let error = $state('');
  let isSaving = $state(false);

  // Coordenadas lógicas de selección dentro del lienzo
  let isDragging = $state(false);
  let startX = $state(0);
  let startY = $state(0);
  let currentX = $state(0);
  let currentY = $state(0);
  let hasSelection = $state(false);

  // Teclado
  let isAnchorLocked = $state(false);

  // Referencias DOM
  let containerEl = $state<HTMLElement>();
  let bgImageEl = $state<HTMLImageElement>();
  let loupeCanvas = $state<HTMLCanvasElement>();

  // Lupa de precisión
  let loupeVisible = $state(true);
  let loupeX = $state(0);
  let loupeY = $state(0);
  let currentColorHex = $state('#000000');

  // Coordenadas calculadas
  const selX = $derived(Math.min(startX, currentX));
  const selY = $derived(Math.min(startY, currentY));
  const selW = $derived(Math.abs(currentX - startX));
  const selH = $derived(Math.abs(currentY - startY));

  const scaleFactor = $derived(monitor?.scale_factor ?? window.devicePixelRatio ?? 1.0);

  const physicalW = $derived(Math.floor(selW * scaleFactor));
  const physicalH = $derived(Math.floor(selH * scaleFactor));

  onMount(async () => {
    if (!isTauri()) { error = 'La captura está disponible en la aplicación de escritorio.'; return; }
    try {
      const monitors = await invoke<MonitorInfo[]>('get_monitors');
      monitor = monitors.find((m) => m.id === monitorId) ?? null;
      if (!monitor) error = 'No se encontró el monitor. Cancela y vuelve a capturar.';
    } catch (err) {
      error = errorMessage(err);
    }
  });

  function updateLoupe(clientX: number, clientY: number) {
    if (!bgImageEl || !imgLoaded || !loupeCanvas) return;

    const offset = 20;
    const loupeSize = 130;
    let lx = clientX + offset;
    let ly = clientY + offset;

    if (lx + loupeSize > window.innerWidth) {
      lx = clientX - loupeSize - offset;
    }
    if (ly + loupeSize > window.innerHeight) {
      ly = clientY - loupeSize - offset;
    }

    loupeX = Math.max(0, lx);
    loupeY = Math.max(0, Math.min(ly, window.innerHeight - loupeSize - 28));

    const ctx = loupeCanvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;

    const zoom = 4;
    const sampleSize = Math.floor(loupeSize / zoom);
    const halfSample = Math.floor(sampleSize / 2);

    const imgW = bgImageEl.naturalWidth;
    const imgH = bgImageEl.naturalHeight;
    const rect = bgImageEl.getBoundingClientRect();

    const scaleX = imgW / rect.width;
    const scaleY = imgH / rect.height;

    const srcX = (clientX - rect.left) * scaleX - halfSample;
    const srcY = (clientY - rect.top) * scaleY - halfSample;

    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, loupeSize, loupeSize);

    try {
      ctx.drawImage(
        bgImageEl,
        srcX,
        srcY,
        sampleSize,
        sampleSize,
        0,
        0,
        loupeSize,
        loupeSize
      );

      const centerPixel = ctx.getImageData(
        Math.floor(loupeSize / 2),
        Math.floor(loupeSize / 2),
        1,
        1
      ).data;
      currentColorHex = `#${[centerPixel[0], centerPixel[1], centerPixel[2]]
        .map((x) => x.toString(16).padStart(2, '0'))
        .join('')
        .toUpperCase()}`;
    } catch {
      // Ignorar errores de carga inicial
    }
  }

  function handlePointerDown(e: PointerEvent) {
    if (isSaving || !imgLoaded || !monitor || (e.target as HTMLElement).closest('button')) return;
    if (e.button === 2) {
      cancel();
      return;
    }
    if (e.button !== 0) return;
    containerEl?.setPointerCapture(e.pointerId);
    isAnchorLocked = false;

    isDragging = true;
    startX = e.clientX;
    startY = e.clientY;
    currentX = e.clientX;
    currentY = e.clientY;
    hasSelection = true;
    updateLoupe(e.clientX, e.clientY);
  }

  function handlePointerMove(e: PointerEvent) {
    if (isDragging || !hasSelection) {
      currentX = Math.max(0, Math.min(e.clientX, window.innerWidth));
      currentY = Math.max(0, Math.min(e.clientY, window.innerHeight));
    }
    updateLoupe(e.clientX, e.clientY);
  }

  async function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    currentX = Math.max(0, Math.min(e.clientX, window.innerWidth));
    currentY = Math.max(0, Math.min(e.clientY, window.innerHeight));
    containerEl?.releasePointerCapture(e.pointerId);

    if (selW >= 4 && selH >= 4) {
      await commitCrop();
    } else {
      hasSelection = false;
    }
  }

  async function commitCrop() {
    if (isSaving || !monitor || !imgLoaded || selW < 4 || selH < 4) return;
    isSaving = true;
    error = '';

    const physX = Math.floor(selX * scaleFactor);
    const physY = Math.floor(selY * scaleFactor);
    const physW = Math.max(1, Math.floor(selW * scaleFactor));
    const physH = Math.max(1, Math.floor(selH * scaleFactor));

    try {
      const selection: CropSelection = { monitor_id: monitorId, physical_x: physX, physical_y: physY, physical_w: physW, physical_h: physH };
      await invoke('confirm_crop', { selection });
    } catch (err) {
      error = errorMessage(err);
    } finally {
      isSaving = false;
    }
  }

  async function cancel() {
    try {
      await invoke('cancel_capture');
    } catch (err) {
      error = errorMessage(err);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.repeat && e.key === 'Enter') return;
    if (e.key === 'Escape') {
      cancel();
      return;
    }

    const step = e.shiftKey ? 1 : 10;

    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
      e.preventDefault();
      if (!isAnchorLocked) {
        if (e.key === 'ArrowLeft') currentX = Math.max(0, currentX - step);
        if (e.key === 'ArrowRight') currentX = Math.min(window.innerWidth, currentX + step);
        if (e.key === 'ArrowUp') currentY = Math.max(0, currentY - step);
        if (e.key === 'ArrowDown') currentY = Math.min(window.innerHeight, currentY + step);
        startX = currentX;
        startY = currentY;
      } else {
        if (e.key === 'ArrowLeft') currentX = Math.max(0, currentX - step);
        if (e.key === 'ArrowRight') currentX = Math.min(window.innerWidth, currentX + step);
        if (e.key === 'ArrowUp') currentY = Math.max(0, currentY - step);
        if (e.key === 'ArrowDown') currentY = Math.min(window.innerHeight, currentY + step);
        hasSelection = true;
      }
      updateLoupe(currentX, currentY);
    } else if (e.key === ' ' || e.code === 'Space') {
      e.preventDefault();
      if (!isAnchorLocked) {
        isAnchorLocked = true;
        startX = currentX;
        startY = currentY;
        hasSelection = true;
      } else {
        isAnchorLocked = false;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (hasSelection && selW >= 4 && selH >= 4) {
        commitCrop();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main
  class="overlay-container"
  bind:this={containerEl}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={() => { isDragging = false; hasSelection = false; }}
  oncontextmenu={(e) => e.preventDefault()}
  role="region"
  aria-label="Lienzo de selección de captura"
>
  <!-- Fotograma congelado del monitor -->
  <img
    class="frozen-bg"
    bind:this={bgImageEl}
    src={isTauri() ? assetUrl(`monitor-${monitorId}`) : undefined}
    crossorigin="anonymous"
    alt="Pantalla congelada para recorte"
    draggable="false"
    onerror={() => { error = 'No se pudo cargar la pantalla. Cancela e inténtalo de nuevo.'; }}
    onload={() => {
      imgLoaded = true;
      updateLoupe(window.innerWidth / 2, window.innerHeight / 2);
    }}
  />

  <!-- Velo de oscurecimiento exterior -->
  {#if !hasSelection}<div class="dim-backdrop"></div>{/if}

  <!-- Caja de selección recortada -->
  {#if hasSelection && selW > 0 && selH > 0}
    <div
      class="selection-box"
      style:left="{selX}px"
      style:top="{selY}px"
      style:width="{selW}px"
      style:height="{selH}px"
    >
      <div class="corner-mark top-left"></div>
      <div class="corner-mark top-right"></div>
      <div class="corner-mark bottom-left"></div>
      <div class="corner-mark bottom-right"></div>

      <div class="dimension-badge" class:badge-inside={selY < 32}>
        <span class="dim-text">{physicalW} × {physicalH} px</span>
      </div>
    </div>
  {/if}

  <!-- Lupa de inspección óptica -->
  {#if loupeVisible && imgLoaded}
    <div
      class="optical-loupe"
      style:left="{loupeX}px"
      style:top="{loupeY}px"
    >
      <div class="loupe-lens">
        <canvas
          bind:this={loupeCanvas}
          width="130"
          height="130"
          class="loupe-canvas"
        ></canvas>
        <div class="loupe-reticle"></div>
      </div>
      <div class="loupe-meta">
        <span class="loupe-coord">
          {Math.round(currentX * scaleFactor)}, {Math.round(currentY * scaleFactor)}
        </span>
        <span class="loupe-color">
          {currentColorHex}
        </span>
      </div>
    </div>
  {/if}

  <div class="shortcut-hints">
    <span><strong>Arrastrar</strong>: Delimitar área</span>
    <span><strong>Esc / Clic Der.</strong>: Cancelar</span>
    <span><strong>Flechas</strong>: Ajuste fino</span>
    <span><strong>Espacio</strong>: Fijar ancla</span>
    <span><strong>Enter</strong>: Confirmar</span>
  </div>
  {#if error}<div class="capture-error" role="alert">{error}<button onclick={cancel}>Cancelar</button></div>{/if}
  {#if isSaving}<div class="capture-saving" role="status">Guardando recorte…</div>{/if}
</main>

<style>
  .overlay-container {
    touch-action: none;
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    cursor: crosshair;
    user-select: none;
    overflow: hidden;
  }

  .frozen-bg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: fill;
    display: block;
    pointer-events: none;
  }

  .dim-backdrop {
    position: absolute;
    inset: 0;
    background-color: rgba(20, 20, 19, 0.42);
    pointer-events: none;
  }

  .selection-box {
    position: absolute;
    box-sizing: border-box;
    border: 1px solid #F4F3EF;
    outline: 1px solid #141413;
    outline-offset: -2px;
    box-shadow: 0 0 0 9999px rgba(20, 20, 19, 0.42);
    pointer-events: none;
  }

  .corner-mark {
    position: absolute;
    width: 8px;
    height: 8px;
    border-color: #D95D1E;
  }

  .corner-mark.top-left {
    top: -2px;
    left: -2px;
    border-top: 2px solid #D95D1E;
    border-left: 2px solid #D95D1E;
  }

  .corner-mark.top-right {
    top: -2px;
    right: -2px;
    border-top: 2px solid #D95D1E;
    border-right: 2px solid #D95D1E;
  }

  .corner-mark.bottom-left {
    bottom: -2px;
    left: -2px;
    border-bottom: 2px solid #D95D1E;
    border-left: 2px solid #D95D1E;
  }

  .corner-mark.bottom-right {
    bottom: -2px;
    right: -2px;
    border-bottom: 2px solid #D95D1E;
    border-right: 2px solid #D95D1E;
  }

  .dimension-badge {
    position: absolute;
    bottom: -26px;
    left: 0;
    background: #141413;
    color: #F4F3EF;
    border: 1px solid #262624;
    padding: 2px 6px;
    border-radius: 2px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    white-space: nowrap;
  }

  .dimension-badge.badge-inside {
    bottom: auto;
    top: 4px;
    left: 4px;
  }

  .optical-loupe {
    position: absolute;
    width: 130px;
    background: #141413;
    border: 1px solid #4A4945;
    border-radius: 2px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);
    pointer-events: none;
    z-index: 9999;
    overflow: hidden;
  }

  .loupe-lens {
    position: relative;
    width: 130px;
    height: 130px;
  }

  .loupe-canvas {
    display: block;
    width: 130px;
    height: 130px;
  }

  .loupe-reticle {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 8px;
    height: 8px;
    transform: translate(-50%, -50%);
    border: 1px solid #D95D1E;
    box-sizing: border-box;
  }

  .loupe-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 6px;
    background: #262624;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: #A3A19A;
  }

  .loupe-color {
    font-weight: bold;
  }

  .shortcut-hints {
    flex-wrap: wrap;
    justify-content: center;
    max-width: calc(100vw - 24px);
    width: max-content;
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(20, 20, 19, 0.88);
    border: 1px solid #262624;
    border-radius: 2px;
    padding: 4px 12px;
    display: flex;
    gap: 16px;
    color: #A3A19A;
    font-size: 12px;
    pointer-events: none;
    z-index: 1000;
  }

  .shortcut-hints strong {
    color: #F4F3EF;
  }
  .capture-error, .capture-saving { position: fixed; top: 24px; left: 50%; transform: translateX(-50%); max-width: calc(100vw - 48px); padding: 12px 16px; background: #141413; border: 1px solid #ff8a50; color: #f4f3ef; font-size: 13px; z-index: 10001; }
  .capture-error button { margin-left: 12px; color: #ff8a50; background: none; border: 0; text-decoration: underline; cursor: pointer; }
</style>
