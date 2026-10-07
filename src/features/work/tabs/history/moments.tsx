import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { Moment } from '@/lib/api/types'
import { formatTotal } from '@/lib/format'
import { allOf, labelOf, useProfile, useVocabulary } from '@/lib/useProfile'
import { Chip } from '@/components/ui/chip'
import { destinationOf, JournalChip, sentence } from '@/features/journal/JournalFeed'

/** How one moment of a work's axis is said: its words, where it leads, and
 *  the chip that names what kind of thing it is. */
export interface SaidMoment {
  text: string
  /** Where a press on it goes; `null` for a moment that leads nowhere. */
  to: string | null
  chip: ReactNode
}

/**
 * The words for the moments of one work's axis (ADR 0056), built now rather
 * than stored, like the journal's: a version is named by its role in the
 * window's language, a release by the door it goes out through.
 *
 * Every moment leads to where it can be read whole - a version opens on the
 * Versions tab, a score on the Score tab, a publication on its own card - so
 * reading "v4, from v2" and then hunting for v4 is not a walk anyone takes.
 */
export function useSayMoment(workId: string): (moment: Moment) => SaidMoment {
  const { t } = useTranslation()
  const { config } = useProfile()
  const vocabulary = useVocabulary(workId)

  return (moment) => {
    switch (moment.type) {
      case 'begun':
        return { text: t('timeline.begun'), to: null, chip: null }
      case 'version': {
        const role = labelOf(vocabulary.version_roles, moment.role)
        const name = moment.label === null ? '' : ` · ${moment.label}`
        const from =
          moment.parent === null ? '' : ` · ${t('timeline.from', { revision: moment.parent })}`
        return {
          text: `${role} v${moment.revision}${name}${from}`,
          to: `/works/${workId}/versions?version=${moment.id}`,
          chip: <Chip variant="accent">{t('timeline.kinds.version')}</Chip>,
        }
      }
      case 'score': {
        const tier = moment.tier === null ? '' : ` · ${labelOf(vocabulary.tiers, moment.tier)}`
        const read =
          moment.role === null || moment.revision === null
            ? ''
            : ` · ${labelOf(vocabulary.version_roles, moment.role)} v${moment.revision}`
        const by = moment.rater === null ? '' : ` · ${t('timeline.by', { rater: moment.rater })}`
        return {
          text: `${t('timeline.scored', { total: formatTotal(moment.total) })}${tier}${read}${by}`,
          to: `/works/${workId}/score`,
          chip: <Chip variant="good">{t('timeline.kinds.score')}</Chip>,
        }
      }
      case 'made':
        return {
          text: t(moment.depth > 1 ? 'timeline.madeVia' : 'timeline.made', {
            title: moment.title,
          }),
          to: `/works/${moment.work_id}`,
          chip: <Chip variant="outline">{labelOf(config.work_kinds, moment.kind)}</Chip>,
        }
      case 'release': {
        const door = labelOf(allOf(config, 'release_kinds'), moment.kind)
        return {
          text: t(moment.released ? 'timeline.released' : 'timeline.booked', {
            title: moment.title,
            door,
          }),
          to: `/works/${moment.work_id}`,
          chip: <Chip variant={moment.released ? 'good' : 'warn'}>{door}</Chip>,
        }
      }
      case 'entry':
        return {
          text: sentence(moment.entry, t),
          to: destinationOf(moment.entry),
          chip: <JournalChip entry={moment.entry} />,
        }
    }
  }
}
