import init from '@/wasm/pkg/uno_wasm'

let initPromise: Promise<void> | null = null

export function ensureWasmInit(): Promise<void> {
  if (!initPromise) {
    initPromise = init().then(() => undefined)
  }
  return initPromise
}
