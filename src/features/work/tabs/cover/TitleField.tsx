import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { DraftText } from '@/features/cover/Section'

interface Props {
  /** As the cover holds it: absent follows what the work is made from,
      empty is no title at all. */
  title: string | null
  /** The title of what the publication is made from - the song's. */
  sourceTitle: string
  disabled?: boolean
  onChange: (title: string | null) => void
}

/**
 * The words lettered on the cover.
 *
 * They are the song's, not the publication's: "the song - clip" is a name
 * for the catalogue, and nobody letters it on a picture. So the title
 * follows what the work is made from until it is written here - and a title
 * written back to the song's follows it again. A cover can also go without
 * one, its captions alone.
 */
export function TitleField({ title, sourceTitle, disabled, onChange }: Props) {
  const { t } = useTranslation()
  const none = title === ''
  return (
    <div className="flex flex-col gap-1.5">
      {!none && (
        <DraftText
          single
          label={t('cover.lettering.words')}
          value={title ?? sourceTitle}
          placeholder={sourceTitle}
          disabled={disabled}
          onCommit={(text) => onChange(text.trim() === sourceTitle.trim() ? null : text)}
        />
      )}
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <Checkbox
          checked={none}
          disabled={disabled}
          onCheckedChange={(off) => onChange(off ? '' : null)}
        >
          {t('cover.lettering.noTitle')}
        </Checkbox>
        {title !== null && title !== '' && (
          <Button variant="link" disabled={disabled} onClick={() => onChange(null)}>
            {t('cover.lettering.followSource')}
          </Button>
        )}
      </div>
    </div>
  )
}
