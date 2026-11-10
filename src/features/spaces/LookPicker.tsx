import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { SPACE_COLOURS, SPACE_MARKS, SpaceGlyph } from '@/features/spaces/look'
import { cn } from '@/lib/utils'

interface Props {
  colour: string
  mark: string
  onChange: (next: { colour?: string; mark?: string }) => void
}

/** A space's colour and mark, each one press from the sets of `look`. */
export function LookPicker({ colour, mark, onChange }: Props) {
  const { t } = useTranslation()

  return (
    <div className="flex flex-col gap-3">
      <FieldGroup label={t('spaces.colour')}>
        <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('spaces.colour')}>
          {SPACE_COLOURS.map((entry) => (
            <Button
              key={entry}
              variant="icon"
              size="icon-sm"
              aria-label={entry}
              aria-pressed={entry === colour}
              className={cn(entry === colour && 'ring-2 ring-accent')}
              onClick={() => onChange({ colour: entry })}
            >
              <span aria-hidden className="size-4 rounded-sm" style={{ backgroundColor: entry }} />
            </Button>
          ))}
        </div>
      </FieldGroup>
      <FieldGroup label={t('spaces.mark')}>
        <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('spaces.mark')}>
          {Object.entries(SPACE_MARKS).map(([key, Icon]) => (
            <Button
              key={key}
              variant="icon"
              size="icon-sm"
              aria-label={t(`spaces.marks.${key}`)}
              aria-pressed={key === mark}
              className={cn(key === mark && 'bg-accent-soft text-accent')}
              onClick={() => onChange({ mark: key })}
            >
              <Icon aria-hidden />
            </Button>
          ))}
        </div>
      </FieldGroup>
      <p className="flex items-center gap-2 text-sm text-dim">
        <SpaceGlyph space={{ colour, mark, kind: null }} size="md" />
        {t('spaces.lookPreview')}
      </p>
    </div>
  )
}
