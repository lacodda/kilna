import { useTranslation } from 'react-i18next'

const STEPS = ['work', 'versions', 'score', 'slot', 'shipped'] as const

// The loop the whole product is built around, shown as the shape of the app.
// Steps beyond versions are not built yet and read as dimmed.
const BUILT = 3

export function LoopBar() {
  const { t } = useTranslation()

  return (
    <ol className="flex flex-wrap items-center gap-1.5 text-xs">
      {STEPS.map((step, index) => (
        <li key={step} className="flex items-center gap-1.5">
          <span
            className={
              index < BUILT
                ? 'rounded-full border border-kiln-500/60 px-2 py-0.5 text-kiln-700 dark:text-kiln-200'
                : 'rounded-full border border-neutral-300 px-2 py-0.5 text-neutral-400 dark:border-neutral-700'
            }
          >
            {t(`loop.${step}`)}
          </span>
          {index < STEPS.length - 1 && (
            <span aria-hidden className="text-neutral-400">
              →
            </span>
          )}
        </li>
      ))}
    </ol>
  )
}
