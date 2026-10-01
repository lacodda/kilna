import { useTranslation } from 'react-i18next'
import type { ProfileConfig } from '@/lib/api/types'
import { MOST_IDEAS, ideasOnMake } from '@/lib/ideas'
import { FieldGroup } from '@/components/ui/field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'

/**
 * How many ideas for its cover a publication is given when it is made
 * (v0.89): "Make a clip" starts them beside the release meta. Three unless
 * the profile says otherwise; 0 asks for none. Offered only where a kind has
 * a cover to have ideas for.
 */
export function CoverIdeasEditor({
  config,
  onChange,
}: {
  config: ProfileConfig
  onChange: (count: number) => void
}) {
  const { t } = useTranslation()
  if (!config.work_kinds.some((kind) => kind.cover === true)) return null

  return (
    <FieldGroup label={t('editor.coverIdeas')} help={t('editor.coverIdeasHint')}>
      <SegmentedControl
        aria-label={t('editor.coverIdeas')}
        value={String(ideasOnMake(config))}
        onValueChange={(next) => onChange(Number(next))}
      >
        {Array.from({ length: MOST_IDEAS + 1 }, (_, n) => (
          <Segment key={n} value={String(n)}>
            {n}
          </Segment>
        ))}
      </SegmentedControl>
    </FieldGroup>
  )
}
