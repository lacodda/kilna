import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import type { SchemaCustomProps } from '@/components/ui/schema-fields'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

/** A point of an emotional arc: a section, its feeling, how strong. */
export interface ArcPoint {
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

const WIDTH = 240
const HEIGHT = 56

/**
 * The emotional arc of a work (v0.146, FR-10 10.1): section by section, what
 * it feels and how strongly - a row each, and the curve the intensities draw
 * above them. The assistant reads it in order, with the work's other fields.
 */
export function ArcEditor({ field, value, onValueChange, disabled }: SchemaCustomProps) {
  const { t } = useTranslation()
  const points = arcPointsOf(value)
  const set = (next: ArcPoint[]) => onValueChange(next.length === 0 ? undefined : next)
  const change = (at: number, patch: Partial<ArcPoint>) =>
    set(points.map((one, index) => (index === at ? { ...one, ...patch } : one)))
  const step = points.length > 1 ? WIDTH / (points.length - 1) : 0
  const curve = points
    .map((one, index) => {
      const x = points.length === 1 ? WIDTH / 2 : index * step
      const y = HEIGHT - 4 - ((one.intensity - 1) / 9) * (HEIGHT - 8)
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')

  return (
    <div className="col-span-full flex flex-col gap-1.5" role="group" aria-label={field.label}>
      <span className="caption">{field.label}</span>
      {points.length > 1 && (
        <svg
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          className="h-14 w-full max-w-sm"
          role="img"
          aria-label={t('work.arcCurve', {
            list: points.map((one) => `${one.section} ${one.intensity}`).join(', '),
          })}
        >
          <polyline points={curve} className="fill-none stroke-accent" strokeWidth={2} />
          {curve.split(' ').map((spot, index) => {
            const [x, y] = spot.split(',')
            return <circle key={String(index)} cx={x} cy={y} r={3} className="fill-accent" />
          })}
        </svg>
      )}
      {points.map((one, index) => (
        <div key={String(index)} className="flex items-center gap-1.5">
          <Input
            value={one.section}
            disabled={disabled}
            placeholder={t('work.arcSection')}
            aria-label={t('work.arcSectionOf', { n: index + 1 })}
            onChange={(event) => change(index, { section: event.target.value })}
            className="w-32"
          />
          <Input
            value={one.emotion}
            disabled={disabled}
            placeholder={t('work.arcEmotion')}
            aria-label={t('work.arcEmotionOf', { n: index + 1 })}
            onChange={(event) => change(index, { emotion: event.target.value })}
            className="min-w-0 flex-1"
          />
          <Input
            type="number"
            min={1}
            max={10}
            value={String(one.intensity)}
            disabled={disabled}
            aria-label={t('work.arcIntensityOf', { n: index + 1 })}
            onChange={(event) =>
              change(index, { intensity: clampIntensity(Number(event.target.value) || 1) })
            }
            className="w-16"
          />
          <Button
            variant="icon"
            size="icon-sm"
            disabled={disabled}
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
        disabled={disabled}
        onClick={() => set([...points, { section: '', emotion: '', intensity: 5 }])}
      >
        <Plus aria-hidden />
        {t('work.arcAdd')}
      </Button>
    </div>
  )
}
