const REMOTE_ROOM_KEY = 'uno-remote-pending-room'

let inMemoryRemoteRoom: string | null = null

/** 記錄即將進入的連線房間代碼（供 /game 建立 RemoteGameClient）。 */
export function savePendingRemoteRoom(code: string): void {
  inMemoryRemoteRoom = code
  try {
    sessionStorage.setItem(REMOTE_ROOM_KEY, code)
  } catch {
    // storage unavailable: in-memory only
  }
}

export function loadPendingRemoteRoom(): string | null {
  if (inMemoryRemoteRoom) return inMemoryRemoteRoom
  try {
    const raw = sessionStorage.getItem(REMOTE_ROOM_KEY)
    if (raw) inMemoryRemoteRoom = raw
    return raw
  } catch {
    return null
  }
}

export function clearPendingRemoteRoom(): void {
  inMemoryRemoteRoom = null
  try {
    sessionStorage.removeItem(REMOTE_ROOM_KEY)
  } catch {
    // ignore
  }
}
