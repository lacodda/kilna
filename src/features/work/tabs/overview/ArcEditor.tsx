import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import { useFieldDraft } from '@/components/ui/field-draft'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

/**
 * A point of an emotional arc: a section, its feeling, how strong. A type,
 * not an interface: it is stored as it stands, and only a type literal is a
 * JSON value to the compiler.
 */
export type ArcPoint = {
  section: string
  emotion: string
  intensity: number
}

/** The points a stored value holds, intensity kept from 1 to 10. */
export function arcPointsOf(value: unknown): ArcPoint[] {
  if (!Array.isArray(value)) return []
  return value
    .filter((one): one is Record<string, unknown> => typeof one === 'object' && one !== null)
    .map((one) => ({
      section: typeof one.section === 'string' ? one.section : '',
      emotion: typeof one.emotion === 'string' ? one.emotion : '',
      intensity: clampIntensity(typeof one.intensity === 'number' ? one.intensity : 5),
    }))
}

function clampIntensity(value: number): number {
  return Math.min(10, Math.max(1, Math.round(value)))
}

/** Whether a typed intensity reads as a whole number from 1 to 10. */
function readableIntensity(text: string): boolean {
  const number = Number(text.trim())
  return text.trim() !== '' && Number.isInteger(number) && number >= 1 && number <= 10
}

const WIDTH = 240
const HEIGHT = 56

/**
 * The emotional arc of a work (v0.146, FR-10 10.1): section by section, what
 * it feels and how strongly - a row each, and the curve the intensities draw
 * above them. It stands with the paragraphs, at their width: in the grid of
 * short fields its row was cut off. Every box keeps the rules of a field
 * (`useFieldDraft`) - leaving it writes only a change, Escape puts back what
 * is stored - so typing a word is one entry of the history, not one a key.
 */
export function ArcEditor({
  label,
  value,
  onChange,
}: {
  label: string
  value: unknown
  onChange: (points: ArcPoint[] | null) => void
}) {
  const { t } = useTranslation()
  const points = arcPointsOf(value)
  const set = (next: ArcPoint[]) => onChange(next.length === 0 ? null : next)
  const change = (at: number, patch: Partial<ArcPoint>) =>
    set(points.map((one, index) => (index === at ? { ...one, ...patch } : one)))
  const step = points.length > 1 ? WIDTH / (points.length - 1) : 0
  // Whole units of the drawing's own box: a coordinate, never a number said.
  const spots = points.map((one, index) => ({
    x: Math.round(points.length === 1 ? WIDTH / 2 : index * step),
    y: Math.round(HEIGHT - 4 - ((one.intensity - 1) / 9) * (HEIGHT - 8)),
  }))

  return (
    <div className="flex flex-col gap-1.5" role="group" aria-label={label}>
      {points.length > 1 && (
        <svg
          viewBox={`-4 0 ${String(WIDTH + 8)} ${String(HEIGHT)}`}
          className="h-14 w-full max-w-md"
          role="img"
          aria-label={t('work.arcCurve', {
            list: points.map((one) => `${one.section} ${String(one.intensity)}`).join(', '),
          })}
        >
          <polyline
            points={spots.map((spot) => `${String(spot.x)},${String(spot.y)}`).join(' ')}
            className="fill-none stroke-accent"
            strokeWidth={2}
          />
          {spots.map((spot, index) => (
            <circle key={String(index)} cx={spot.x} cy={spot.y} r={3} className="fill-accent" />
          ))}
        </svg>
      )}
      {points.map((one, index) => (
        <div key={String(index)} className="flex items-center gap-1.5">
          <ArcBox
            className="w-36"
            value={one.section}
            placeholder={t('work.arcSection')}
            label={t('work.arcSectionOf', { n: index + 1 })}
            onCommit={(section) => change(index, { section: section.trim() })}
          />
          <ArcBox
            className="min-w-0 flex-1"
            value={one.emotion}
            placeholder={t('work.arcEmotion')}
            label={t('work.arcEmotionOf', { n: index + 1 })}
            onCommit={(emotion) => change(index, { emotion: emotion.trim() })}
          />
          <ArcBox
            className="w-16"
            value={String(one.intensity)}
            label={t('work.arcIntensityOf', { n: index + 1 })}
            readable={readableIntensity}
            onCommit={(text) => change(index, { intensity: clampIntensity(Number(text)) })}
          />
          <Button
            variant="icon"
            size="icon-sm"
            aria-label={t('work.arcRemove', { n: index + 1 })}
            onClick={() => set(points.filter((_, at) => at !== index))}
          >
            <X aria-hidden />
          </Button>
        </div>
      ))}
      <Button
        size="xs"
        variant="ghost"
        className="self-start"
        onClick={() => set([...points, { section: '', emotion: '', intensity: 5 }])}
      >
        <Plus aria-hidden />
        {t('work.arcAdd')}
      </Button>
    </div>
  )
}

/** One box of a row, written when it is left. */
function ArcBox({
  value,
  label,
  placeholder,
  className,
  readable,
  onCommit,
}: {
  value: string
  label: string
  placeholder?: string
  className: string
  readable?: (text: string) => boolean
  onCommit: (text: string) => void
}) {
  const draft = useFieldDraft(value, onCommit, { readable })
  return <Input aria-label={label} placeholder={placeholder} className={className} {...draft} />
}
