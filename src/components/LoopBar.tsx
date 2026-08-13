import { useTranslation } from 'react-i18next'
import { ArrowRight } from 'lucide-react'

const STEPS = ['work', 'versions', 'score', 'slot', 'shipped'] as const

// The loop the whole product is built around, shown as the shape of the app.
// Every step is built as of v0.4.0; the constant stays so a future step added
// ahead of its implementation reads as dimmed rather than as a lie.
const BUILT = 5

export function LoopBar() {
  const { t } = useTranslation()

  return (
    <ol className="flex flex-wrap items-center gap-1.5 text-xs">
      {STEPS.map((step, index) => (
        <li key={step} className="flex items-center gap-1.5">
          <span
            className={
              index < BUILT
                ? 'rounded-full border border-accent/60 px-2 py-0.5 text-accent-2'
                : 'rounded-full border border-line px-2 py-0.5 text-faint'
            }
          >
            {t(`loop.${step}`)}
          </span>
          {index < STEPS.length - 1 && <ArrowRight aria-hidden className="size-3.5 text-faint" />}
        </li>
      ))}
    </ol>
  )
}
