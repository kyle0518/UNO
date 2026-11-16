import { useTranslation } from '@/i18n'

export interface DialogUnoButtonProps {
  visible?: boolean
  called?: boolean
  onCall?: () => void
}

export function DialogUnoButton({
  visible = false,
  called = false,
  onCall,
}: DialogUnoButtonProps) {
  const { t } = useTranslation()
  if ((!visible && !called) || !onCall) return null

  if (called) {
    return (
      <div
        role="status"
        className="w-full rounded-xl border border-emerald-400/40 bg-emerald-500/15 px-4 py-2 text-center text-sm font-bold text-emerald-300"
      >
        ✓ {t('game.unoCalledDone')}
      </div>
    )
  }

  return (
    <button
      type="button"
      onClick={onCall}
      className="min-h-11 w-full rounded-xl bg-gradient-to-r from-red-500 via-orange-500 to-amber-500 px-4 py-2.5 text-sm font-black tracking-wider text-white shadow-lg transition hover:brightness-110 active:scale-95"
    >
      {t('game.callUnoAction')} (U)
    </button>
  )
}
