import { useEffect, useState } from 'react'
import type { ChatEntry } from '@/game/store'
import { useTranslation } from '@/i18n'

const CHAT_EMOTES = ['thumbs_up', 'laugh', 'angry', 'cry', 'fire', 'uno'] as const

export interface ChatDrawerProps {
  open: boolean
  chat: ChatEntry[]
  onClose: () => void
  onSend: (text: string) => void
  onEmote: (emote: string) => void
}

/** 遊戲中聊天抽屜（手機由底部滑出）＋表情快捷列。 */
export function ChatDrawer({ open, chat, onClose, onSend, onEmote }: ChatDrawerProps) {
  const { t } = useTranslation()
  const [text, setText] = useState('')

  if (!open) return null
  return (
    <div
      role="dialog"
      aria-label={t('room.chatTitle')}
      className="fixed inset-x-0 bottom-0 z-50 mx-auto flex max-h-[62dvh] w-full max-w-lg flex-col gap-3 rounded-t-3xl uno-surface p-4 shadow-2xl"
      style={{ paddingBottom: 'max(1rem, env(safe-area-inset-bottom))' }}
    >
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-bold">{t('room.chatTitle')}</h2>
        <button
          type="button"
          onClick={onClose}
          className="min-h-11 rounded-xl bg-white/10 px-4 text-sm font-semibold"
        >
          {t('game.chatClose')}
        </button>
      </div>
      <div className="flex gap-1.5" aria-label="emote">
        {CHAT_EMOTES.map((e) => (
          <button
            key={e}
            type="button"
            onClick={() => onEmote(e)}
            className="min-h-11 flex-1 rounded-xl bg-white/10 px-1 text-xs font-semibold"
          >
            {e}
          </button>
        ))}
      </div>
      <ul className="flex min-h-24 flex-1 flex-col gap-1 overflow-y-auto text-sm">
        {chat.map((line, idx) => (
          <li key={`${line.at}-${idx}`}>
            <span className="font-bold">{line.from}</span>: {line.text}
          </li>
        ))}
      </ul>
      <form
        className="flex gap-2"
        onSubmit={(ev) => {
          ev.preventDefault()
          const v = text.trim()
          if (!v) return
          onSend(v)
          setText('')
        }}
      >
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={t('room.chatPlaceholder')}
          maxLength={200}
          className="min-h-11 flex-1 rounded-xl border border-white/15 bg-black/40 px-3 text-sm"
        />
        <button
          type="submit"
          className="min-h-11 rounded-xl bg-btn-blue px-4 text-sm font-bold text-white"
        >
          {t('room.chatSend')}
        </button>
      </form>
    </div>
  )
}

export interface EmoteBubbleProps {
  emotes: { seat: number; emote: string; seq: number }[]
  playerNames: string[]
}

/** 最新表情在畫面上方以泡泡顯示 2 秒（以 seq 當 key 重新掛載計時）。 */
export function EmoteBubble({ emotes, playerNames }: EmoteBubbleProps) {
  const latest = emotes[emotes.length - 1]
  if (!latest) return null
  return <EmoteBubbleInner key={latest.seq} emote={latest} playerNames={playerNames} />
}

function EmoteBubbleInner({
  emote,
  playerNames,
}: {
  emote: { seat: number; emote: string; seq: number }
  playerNames: string[]
}) {
  const [visible, setVisible] = useState(true)

  useEffect(() => {
    const id = window.setTimeout(() => setVisible(false), 2000)
    return () => window.clearTimeout(id)
  }, [])

  if (!visible) return null
  const name = playerNames[emote.seat] ?? `P${emote.seat + 1}`
  return (
    <div
      role="status"
      className="pointer-events-none fixed left-1/2 top-16 z-50 -translate-x-1/2 rounded-full bg-black/75 px-4 py-2 text-sm font-bold text-white shadow-lg"
    >
      {name}: {emote.emote}
    </div>
  )
}
