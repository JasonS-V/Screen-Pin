import { convertFileSrc, isTauri } from '@tauri-apps/api/core';

export { isTauri };

export const isMac =
  typeof navigator !== 'undefined' &&
  (/Mac|iPhone|iPod|iPad/i.test(navigator.platform || '') ||
   /Macintosh|Mac OS X/i.test(navigator.userAgent || ''));

export function assetUrl(path: string): string {
  return convertFileSrc(path, 'pinasset');
}

export function errorMessage(error: unknown): string {
  return typeof error === 'string' ? error : error instanceof Error ? error.message : 'No se pudo completar la acción. Inténtalo de nuevo.';
}
