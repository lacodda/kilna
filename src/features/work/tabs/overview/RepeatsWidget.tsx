import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { keepRepeat, unkeepRepeat } from '@/lib/api/register'
import type { RepeatFinding, Work } from '@/lib/api/types'
import { formatDay } from '@/lib/format'
import { today } from '@/lib/month'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { orderFindings, statusOf } from '@/lib/repeats'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { StatusDot } from '@/components/ui/status-dot'
import { Widget } from '@/features/work/tabs/overview/Widget'

/**
 * What the guard of repeats says about this work's song (ADR 0054): every
 * word it shares with a song out or booked, loudest first.
 *
 * Each finding names the word, the other song - the way to it - the day that
 * song went out or is booked for, and why it counts: a spent term of the
 * register, or a rare word, red when the other song is within the window.
 * "I know, keep it" is the one thing stored about a finding: the word stays
 * drawn here and stops counting toward the song's mark, until "Count it
 * again". It is said of the song, so a clip's overview keeps it for the song
 * it is made from, and names that song in its caption.
 *
 * Not the board's "Needs attention" (`FindingsWidget`), which holds the
 * dashboard's complaints about the work: this is about its words, and the
 * click goes to where they are written. Drawn only while the song has a
 * finding, kept or not (`applies`).
 */
export function RepeatsWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const repeats = useQuery(queries.workRepeats(work.id, today()))

  // The song the findings are about: the work, or what it is made from.
  const songId = repeats.data?.work_id ?? work.id

  const keep = useAppMutation({
    mutationFn: (word: string) => keepRepeat(songId, word),
    failure: 'repeats.keepFailed',
    refresh: refresh.term,
  })
  const unkeep = useAppMutation({
    mutationFn: (word: string) => unkeepRepeat(songId, word),
    failure: 'repeats.unkeepFailed',
    refresh: refresh.term,
  })
  const busy = keep.isPending || unkeep.isPending

  const data = repeats.data ?? null
  if (data === null || data.findings.length === 0) return null

  return (
    <Widget
      caption={
        data.work_id === work.id
          ? t('repeats.caption')
          : t('repeats.captionOf', { title: data.title })
      }
      // The words are changed where they are written.
      to={`/works/${data.work_id}/versions`}
      go={t('overview.toVersions')}
      // Something to decide while one still counts; a list of kept words is
      // a record, and asks for nothing.
      tone={data.level === null ? 'plain' : 'attention'}
    >
      {data.level === null && <p className="text-sm text-dim">{t('repeats.allKept')}</p>}
      <ul className="flex flex-col">
        {orderFindings(data.findings).map((finding) => (
          <FindingRow
            // A word is shared with each song that says it: one row each.
            key={`${finding.word}:${finding.neighbour_id}`}
            finding={finding}
            busy={busy}
            onKeep={() => keep.mutate(finding.word)}
            onUnkeep={() => unkeep.mutate(finding.word)}
          />
        ))}
      </ul>
    </Widget>
  )
}

function FindingRow({
  finding,
  busy,
  onKeep,
  onUnkeep,
}: {
  finding: RepeatFinding
  busy: boolean
  onKeep: () => void
  onUnkeep: () => void
}) {
  const { t } = useTranslation()
  const day = formatDay(finding.day)

  return (
    <li
      data-level={finding.level}
      className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-line py-1.5 first:pt-0 last:border-b-0 last:pb-0"
    >
      <span className={cn('flex min-w-0 flex-1 basis-48 flex-col', finding.kept && 'opacity-60')}>
        <span className="flex min-w-0 items-center gap-1.5 text-sm">
          {/* The level as a dot named in words, so the hue is the emphasis
              and not the message. */}
          <StatusDot status={statusOf(finding.level)} label={t(`repeats.level.${finding.level}`)} />
          <b className="truncate font-semibold">{finding.word}</b>
          <span className="shrink-0 text-xs text-faint" title={t(`repeats.whyHint.${finding.why}`)}>
            {t(`repeats.why.${finding.why}`)}
          </span>
        </span>
        <span className="flex min-w-0 items-center gap-1.5 pl-3.5 text-xs text-dim">
          <Link
            to={`/works/${finding.neighbour_id}`}
            className="truncate text-text no-underline hover:underline"
          >
            {finding.neighbour_title}
          </Link>
          <span className="shrink-0 font-mono text-faint">
            {finding.booked ? t('repeats.booked', { day }) : t('repeats.out', { day })}
          </span>
          {finding.also > 0 && (
            <span className="shrink-0 text-faint">
              {t('repeats.alsoIn', { count: finding.also })}
            </span>
          )}
        </span>
      </span>
      <span className="flex shrink-0 items-center gap-1.5">
        {finding.kept && <Badge variant="soft">{t('repeats.kept')}</Badge>}
        {finding.kept ? (
          <Button size="xs" disabled={busy} title={t('repeats.unkeepHint')} onClick={onUnkeep}>
            {t('repeats.unkeep')}
          </Button>
        ) : (
          <Button size="xs" disabled={busy} title={t('repeats.keepHint')} onClick={onKeep}>
            {t('repeats.keep')}
          </Button>
        )}
      </span>
    </li>
  )
}
