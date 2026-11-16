import init from '@/wasm/pkg/uno_wasm'

let initPromise: Promise<void> | null = null

export function ensureWasmInit(): Promise<void> {
  if (initPromise) return initPromise
  const pending = init()
    .then(() => undefined)
    .catch((err: unknown) => {
      if (initPromise === pending) {
        initPromise = null
      }
      throw err
    })
  initPromise = pending
  return pending
}
