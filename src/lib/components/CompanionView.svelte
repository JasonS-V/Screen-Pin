<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  async function triggerCapture() {
    try {
      await invoke('trigger_capture');
    } catch (err) {
      console.error('Error al iniciar captura:', err);
    }
  }

  async function openMemory() {
    try {
      await invoke('open_memory_drawer');
    } catch (err) {
      console.error('Error al abrir memoria:', err);
    }
  }
</script>

<main class="companion-shell">
  <header class="companion-header">
    <div class="logo-mark">
      <div class="optic-reticle"></div>
    </div>
    <div>
      <h1 class="companion-title">Screen-Pin</h1>
      <p class="companion-desc">El recorte que se queda flotando (con memoria)</p>
    </div>
  </header>

  <section class="shortcut-list">
    <div class="shortcut-card">
      <span class="key-combo"><kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd></span>
      <span class="key-desc">Congelar y recortar pantalla</span>
    </div>
    <div class="shortcut-card">
      <span class="key-combo"><kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>M</kbd></span>
      <span class="key-desc">Abrir cajón de Memoria</span>
    </div>
    <div class="shortcut-card">
      <span class="key-combo"><kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>G</kbd></span>
      <span class="key-desc">Alternar Modo Fantasma (Click-Through)</span>
    </div>
  </section>

  <footer class="companion-footer">
    <button type="button" class="action-btn" onclick={openMemory}>
      Ver Memoria
    </button>
    <button type="button" class="action-btn primary" onclick={triggerCapture}>
      Probar recorte ahora
    </button>
  </footer>
</main>

<style>
  .companion-shell {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-height: 100vh;
    padding: 24px;
    background: #141413;
    color: #F4F3EF;
    box-sizing: border-box;
  }

  .companion-header {
    display: flex;
    align-items: center;
    gap: 16px;
    border-bottom: 1px solid #262624;
    padding-bottom: 16px;
  }

  .logo-mark {
    width: 36px;
    height: 36px;
    border: 1px solid #F4F3EF;
    outline: 1px solid #141413;
    outline-offset: -2px;
    border-radius: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #262624;
  }

  .optic-reticle {
    width: 12px;
    height: 12px;
    border-top: 2px solid #D95D1E;
    border-left: 2px solid #D95D1E;
  }

  .companion-title {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .companion-desc {
    font-size: 12px;
    color: #A3A19A;
    margin: 2px 0 0;
  }

  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 24px 0;
  }

  .shortcut-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #262624;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 10px 14px;
  }

  .key-combo {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: #F4F3EF;
  }

  kbd {
    background: #141413;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 2px 6px;
    font-family: inherit;
    font-size: 11px;
    color: #FF8A50;
  }

  .key-desc {
    font-size: 12px;
    color: #A3A19A;
  }

  .companion-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .action-btn {
    background: #262624;
    color: #F4F3EF;
    border: 1px solid #4A4945;
    border-radius: 2px;
    padding: 8px 16px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    cursor: pointer;
    transition: border-color 120ms ease;
  }

  .action-btn.primary {
    border-color: #D95D1E;
  }

  .action-btn:hover {
    border-color: #FF8A50;
  }

  .action-btn:focus-visible {
    outline: 2px solid #FF8A50;
    outline-offset: 1px;
  }
</style>
