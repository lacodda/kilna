import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

/**
 * What the window shows while the workspace is being opened.
 *
 * The same picture `index.html` painted before the bundle arrived - the mark,
 * the name, the promise, the version - so that nothing jumps when React takes
 * over, plus the two lines only the application can say: what it is doing, in
 * the person's language, and one thing worth knowing. The static one is taken
 * down here, on the first render, because from this moment the page is the
 * one drawing.
 */
export function Splash() {
  const { t } = useTranslation()

  useEffect(() => {
    document.getElementById('splash')?.remove()
  }, [])

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed inset-0 flex flex-col items-center justify-center gap-2.5 bg-bg text-text"
    >
      <svg className="mb-1.5 size-14" viewBox="0 0 32 32" aria-hidden>
        <path d="M16 2 28 9v14L16 30 4 23V9z" fill="var(--accent)" />
        <text
          x="16"
          y="20.5"
          fontFamily="Consolas, monospace"
          fontSize="11"
          fontWeight="700"
          fill="var(--on-accent)"
          textAnchor="middle"
        >
          ki
        </text>
      </svg>
      <div className="text-[26px] leading-none font-semibold tracking-[0.02em]">
        {t('app.name')}
      </div>
      <div className="text-[13px] text-dim">{t('app.tagline')}</div>
      <div className="mt-1.5 font-mono text-[11px] text-faint">v{__APP_VERSION__}</div>
      <div className="relative mt-4 h-0.5 w-40 overflow-hidden rounded-full bg-line">
        <div className="splash-sweep absolute top-0 h-full w-2/5 rounded-full bg-accent" />
      </div>
      <p className="mt-3 text-xs text-dim">{t('splash.opening')}</p>
      <p className="text-[11px] text-faint">{t('splash.tip')}</p>
    </div>
  )
}
