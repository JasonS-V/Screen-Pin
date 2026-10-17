import { convertFileSrc, isTauri } from '@tauri-apps/api/core';

export { isTauri };

export function assetUrl(path: string): string {
  return convertFileSrc(path, 'pinasset');
}

export function errorMessage(error: unknown): string {
  return typeof error === 'string' ? error : error instanceof Error ? error.message : 'No se pudo completar la acción. Inténtalo de nuevo.';
}
