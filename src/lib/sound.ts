import { isTauri } from '$lib/desktop';
import { emit, listen } from '@tauri-apps/api/event';

const SOUND_STORAGE_KEY = 'screen_pin_sound_enabled';
export const CLICK_SOUND_DATA_URI = 'data:audio/mp3;base64,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';

let audioCtx: AudioContext | null = null;
let audioBuffer: AudioBuffer | null = null;
let isDecoding = false;
let lastPlayTimestamp = 0;
let fallbackAudio: HTMLAudioElement | null = null;

let soundEnabledCache = true;

if (typeof window !== 'undefined') {
  const stored = localStorage.getItem(SOUND_STORAGE_KEY);
  soundEnabledCache = stored === null ? true : stored === 'true';
}

export function isSoundEnabled(): boolean {
  if (typeof window === 'undefined') return true;
  return soundEnabledCache;
}

export function setSoundEnabled(enabled: boolean): void {
  soundEnabledCache = enabled;
  if (typeof window !== 'undefined') {
    localStorage.setItem(SOUND_STORAGE_KEY, enabled ? 'true' : 'false');
    window.dispatchEvent(new CustomEvent('screen-pin:sound-changed', { detail: enabled }));
    if (isTauri()) {
      void emit('sound:setting-changed', enabled);
    }
  }
}

function getAudioContext(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  if (!audioCtx) {
    const AudioCtxClass = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (AudioCtxClass) {
      audioCtx = new AudioCtxClass();
    }
  }
  return audioCtx;
}

async function prepareAudioBuffer(): Promise<AudioBuffer | null> {
  if (audioBuffer) return audioBuffer;
  if (isDecoding) return null;
  const ctx = getAudioContext();
  if (!ctx) return null;

  isDecoding = true;
  try {
    const res = await fetch(CLICK_SOUND_DATA_URI);
    const arrayBuffer = await res.arrayBuffer();
    audioBuffer = await ctx.decodeAudioData(arrayBuffer);
    return audioBuffer;
  } catch (err) {
    console.warn('[Sound] Fallback al decodificar audio:', err);
    return null;
  } finally {
    isDecoding = false;
  }
}

export function playClickSound(): void {
  if (typeof window === 'undefined') return;
  if (!isSoundEnabled()) return;

  const now = performance.now();
  if (now - lastPlayTimestamp < 25) {
    return;
  }
  lastPlayTimestamp = now;

  const ctx = getAudioContext();
  if (ctx) {
    if (ctx.state === 'suspended') {
      void ctx.resume();
    }

    if (audioBuffer) {
      try {
        const source = ctx.createBufferSource();
        source.buffer = audioBuffer;
        const gainNode = ctx.createGain();
        gainNode.gain.value = 0.85;
        source.connect(gainNode);
        gainNode.connect(ctx.destination);
        source.start(0);
        return;
      } catch (err) {
        console.warn('[Sound] Error en WebAudio source:', err);
      }
    } else {
      void prepareAudioBuffer().then(buf => {
        if (buf && ctx.state !== 'closed') {
          const source = ctx.createBufferSource();
          source.buffer = buf;
          const gainNode = ctx.createGain();
          gainNode.gain.value = 0.85;
          source.connect(gainNode);
          gainNode.connect(ctx.destination);
          source.start(0);
        }
      });
    }
  }

  try {
    if (!fallbackAudio) {
      fallbackAudio = new Audio(CLICK_SOUND_DATA_URI);
    }
    fallbackAudio.currentTime = 0;
    fallbackAudio.volume = 0.85;
    void fallbackAudio.play();
  } catch {
    // Silencioso si el navegador bloquea autoplay
  }
}

export function attachGlobalSoundListener(): () => void {
  if (typeof window === 'undefined') return () => {};

  void prepareAudioBuffer();

  const handlePointerDown = () => {
    const ctx = getAudioContext();
    if (ctx && ctx.state === 'suspended') {
      void ctx.resume();
    }
  };

  const handleClick = (event: MouseEvent) => {
    const target = event.target as HTMLElement | null;
    if (!target) return;

    if (target.closest('[data-no-sound]')) return;

    const interactive = target.closest(
      'button, a, input, select, textarea, [role="button"], [role="checkbox"], [role="switch"], [role="tab"], .button, .action-btn, .chip, .crop-btn, [data-sound]'
    );

    if (interactive) {
      playClickSound();
    }
  };

  const handleStorage = (event: StorageEvent) => {
    if (event.key === SOUND_STORAGE_KEY && event.newValue !== null) {
      soundEnabledCache = event.newValue === 'true';
    }
  };

  window.addEventListener('pointerdown', handlePointerDown, { capture: true, passive: true });
  window.addEventListener('click', handleClick, { capture: true });
  window.addEventListener('storage', handleStorage);

  let unlistenTauri: (() => void) | undefined;
  if (isTauri()) {
    listen<boolean>('sound:setting-changed', (e) => {
      soundEnabledCache = e.payload;
    }).then(fn => {
      unlistenTauri = fn;
    }).catch(() => {});
  }

  return () => {
    window.removeEventListener('pointerdown', handlePointerDown, { capture: true });
    window.removeEventListener('click', handleClick, { capture: true });
    window.removeEventListener('storage', handleStorage);
    unlistenTauri?.();
  };
}
