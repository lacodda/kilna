import { useTranslation } from 'react-i18next'

const STEPS = ['work', 'versions', 'score', 'slot', 'shipped'] as const

// The loop the whole product is built around, shown as the shape of the app.
export function LoopBar() {
  const { t } = useTranslation()

  return (
    <ol className="flex flex-wrap items-center gap-2 text-sm">
      {STEPS.map((step, index) => (
        <li key={step} className="flex items-center gap-2">
          <span className="rounded-full border border-kiln-200 px-3 py-1 text-neutral-700 dark:text-neutral-200">
            {t(`loop.${step}`)}
          </span>
          {index < STEPS.length - 1 && <span aria-hidden className="text-kiln-500">→</span>}
        </li>
      ))}
    </ol>
  )
}
