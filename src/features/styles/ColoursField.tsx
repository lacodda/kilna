import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import { HEX, NO_COLOUR, colourFill } from '@/lib/styleBrick'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

interface Props {
  label: string
  help: string
  colours: string[]
  /** The fewest boxes it keeps: a ground keeps its one colour. */
  min: number
  /** The most it takes: four stops of a gradient, one ground of a sample. */
  max: number
  /** Whether the colours are one paint - a ground's or an accent's - drawn
   * as the gradient they make, rather than a palette of separate swatches. */
  gradient?: boolean
  /** `now` writes at once: a picked colour or a box added is a decision. */
  onChange: (colours: string[], now: boolean) => void
}

/**
 * The colours of a style, one box each: a picker and the `#RRGGBB` it is
 * kept as, side by side, as the background's one colour was edited before
 * v0.90.3. A ground or an accent of several colours is a gradient in their
 * order, and its strip shows the gradient the prompt describes.
 */
export function ColoursField({
  label,
  help,
  colours,
  min,
  max,
  gradient = false,
  onChange,
}: Props) {
  const { t } = useTranslation()
  const boxes =
    colours.length >= min ? colours : [...colours, ...Array<string>(min - colours.length).fill('')]
  const set = (index: number, colour: string, now: boolean) =>
    onChange(
      boxes.map((one, at) => (at === index ? colour : one)),
      now,
    )
  const fill = gradient ? colourFill(boxes) : undefined

  return (
    <FieldGroup label={label} help={help}>
      <div className="flex flex-col gap-2">
        {fill !== undefined && boxes.length > 1 && (
          <span
            aria-hidden
            className="block h-6 rounded-md ring-1 ring-line-2"
            style={{ background: fill }}
          />
        )}
        <ul className="flex flex-col gap-1.5">
          {boxes.map((colour, index) => (
            <li key={index} className="flex items-center gap-2">
              <Input
                type="color"
                aria-label={t('styles.pickColourN', { n: index + 1 })}
                value={HEX.test(colour.trim()) ? colour.trim() : NO_COLOUR}
                onChange={(event) => set(index, event.target.value.toUpperCase(), true)}
                className="h-9 w-14 p-0.5"
              />
              <Input
                value={colour}
                aria-label={t('styles.colourN', { n: index + 1 })}
                onChange={(event) => set(index, event.target.value, false)}
                aria-invalid={colour.trim() !== '' && !HEX.test(colour.trim())}
                placeholder="#RRGGBB"
                className="max-w-32 font-mono"
              />
              {boxes.length > min && (
                <Button
                  variant="icon"
                  size="icon-sm"
                  aria-label={t('styles.removeColour', { n: index + 1 })}
                  title={t('styles.removeColour', { n: index + 1 })}
                  onClick={() =>
                    onChange(
                      boxes.filter((_, at) => at !== index),
                      true,
                    )
                  }
                >
                  <X aria-hidden />
                </Button>
              )}
            </li>
          ))}
        </ul>
        {boxes.length < max && (
          <Button
            variant="ghost"
            size="sm"
            className="self-start"
            // The new box starts as the last colour, so a gradient grows from
            // where it ends rather than from black.
            onClick={() => onChange([...boxes, boxes.at(-1)?.trim() ?? ''], false)}
          >
            <Plus aria-hidden />
            {gradient && boxes.length >= 1 ? t('styles.addStop') : t('styles.addColour')}
          </Button>
        )}
      </div>
    </FieldGroup>
  )
}
