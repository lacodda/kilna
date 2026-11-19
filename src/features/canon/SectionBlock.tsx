import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { ChevronDown, ChevronRight, Plus } from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import { settleSection, unrelateCards } from '@/lib/api/canon'
import type { BankState, CanonSection, CardView, ReadFact, Relation } from '@/lib/api/types'
import { bankState } from '@/lib/bank'
import { factsIn, seenThrough, type LensChoice } from '@/lib/canon'
import { announceEdited } from '@/lib/edited'
import { today } from '@/lib/month'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { RowMenu } from '@/components/RowMenu'
import { useDetailed } from '@/features/canon/disclosure'
import { FactEditor } from '@/features/canon/FactEditor'
import { FactRow } from '@/features/canon/FactRow'
import { QuotaReportDialog } from '@/features/canon/QuotaReportDialog'
import { RelationDialog } from '@/features/canon/RelationDialog'
import { cn } from '@/lib/utils'

interface Props {
  view: CardView
  section: CanonSection
  lens: LensChoice
  onOpen: (id: string) => void
  /** The facts to draw, when they are not the section's own: the facts left
   *  under a section the profile no longer names. */
  facts?: ReadFact[]
  /** Drawn as a panel of the channel's board rather than a stretch of the
   *  card's column. */
  panel?: boolean
  /** Opened from "+ Add section": the way to its first fact is open. */
  startAdding?: boolean
  /** Folded to its heading (v0.142), and the switch that folds it; a
   *  section given none never folds. */
  folded?: boolean
  onFold?: (fold: boolean) => void
}

/** A section longer than this shows its first few until asked (v0.142). */
const LONG_SECTION = 8
const SHOWN_OF_LONG = 5

/**
 * One section of a card, drawn by its shape: statements with the way to add
 * one, the card's relations with the way to draw one, or the works it
 * appears in, counted and never written.
 */
export function SectionBlock({
  view,
  section,
  lens,
  onOpen,
  facts,
  panel = false,
  startAdding = false,
  folded = false,
  onFold,
}: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [adding, setAdding] = useState(startAdding)
  // A long section shows its first five until asked for the rest (v0.142).
  const [all, setAll] = useState(false)
  const [relating, setRelating] = useState(false)
  const [changing, setChanging] = useState<Relation | null>(null)
  // The space's rules checked from where they are written (v0.151), and a
  // bank narrowed to where its items stand today.
  const [checking, setChecking] = useState(false)
  const [standing, setStanding] = useState<BankState | 'all'>('all')
  const client = useQueryClient()
  const unrelate = useAppMutation({
    mutationFn: (id: string) => unrelateCards(id),
    failure: 'canon.relateFailed',
    onSuccess: () =>
      announceEdited({ client, message: t('canon.relationRemoved'), refresh: refresh.canon }),
  })
  const shape = section.shape ?? 'facts'
  const hint = section.hint !== undefined && section.hint !== null ? sayLabel(section.hint) : null
  // What an assistant proposed lands as drafts; a pool of them proposed at
  // once is settled at once, after it is read, rather than one row at a time.
  const settle = useAppMutation({
    mutationFn: () => settleSection(view.card.id, section.key),
    failure: 'canon.settleFailed',
    onSuccess: (count: number) =>
      announceEdited({
        client,
        message: t('canon.sectionSettled', { count }),
        refresh: refresh.canon,
      }),
  })
  const drafts =
    shape === 'relations' || shape === 'appearances' || section.key === ''
      ? 0
      : (facts ?? factsIn(view.facts, section.key)).filter((read) => read.fact.status === 'draft')
          .length

  const held =
    shape === 'relations'
      ? view.relations.filter((relation) => relation.section === section.key).length
      : shape === 'appearances'
        ? view.appearances.length
        : (facts ?? factsIn(view.facts, section.key)).length
  const head = (
    <header className="mb-1 flex flex-wrap items-center gap-2">
      {onFold !== undefined && (
        <Button
          variant="icon"
          size="icon-xs"
          aria-expanded={!folded}
          aria-label={t(folded ? 'canon.unfoldSection' : 'canon.foldSection', {
            section: sayLabel(section.label),
          })}
          onClick={() => onFold(!folded)}
        >
          {folded ? <ChevronRight aria-hidden /> : <ChevronDown aria-hidden />}
        </Button>
      )}
      <b className={cn('font-semibold', panel ? 'text-base' : 'text-sm')}>
        {sayLabel(section.label)}
      </b>
      {onFold !== undefined && held > 0 && (
        <span className="font-mono text-2xs text-faint">{held}</span>
      )}
      {hint !== null && (
        <span
          className={cn(
            'text-xs',
            panel ? 'rounded-sm bg-accent-soft px-1.5 text-accent-2' : 'text-faint',
          )}
        >
          {hint}
        </span>
      )}
      {/* A pool a prompt reads a handful of: the card shows every entry,
          and says so, or a hundred moves read as a hundred things every
          text is handed. */}
      {section.draw != null && (
        <span className="text-2xs text-faint">{t('canon.drawn', { count: section.draw })}</span>
      )}
      {shape === 'quotas' && held > 0 && (
        <Button size="xs" variant="soft" className="ml-auto" onClick={() => setChecking(true)}>
          {t('quota.checkSpace')}
        </Button>
      )}
      {drafts > 1 && (
        <Button
          size="xs"
          variant="ghost"
          className="ml-auto"
          onClick={() => settle.mutate()}
          disabled={settle.isPending}
        >
          {t('canon.settleSection', { count: drafts })}
        </Button>
      )}
    </header>
  )

  let inner
  if (shape === 'relations') {
    const relations = view.relations.filter((relation) => relation.section === section.key)
    inner = (
      <>
        {relations.map((relation) => (
          <div
            key={relation.link.id}
            className={cn(
              'flex items-start gap-2.5 rounded-md px-2 py-1.5 hover:bg-soft/50',
              !seenThrough(relation.lenses, lens) && 'opacity-25',
            )}
          >
            <LayerMark layer={relation.link.layer} />
            <div className="min-w-0 flex-1 text-sm">
              <Button
                variant="link"
                className="font-semibold"
                onClick={() => onOpen(relation.other_id)}
              >
                {relation.other_title ?? t('canon.untitled')}
              </Button>
              {relation.label !== null && <span className="text-dim"> — {relation.label}</span>}
            </div>
            <RowMenu
              label={t('canon.relationActions')}
              actions={[
                {
                  key: 'edit',
                  label: t('canon.editRelation'),
                  onSelect: () => setChanging(relation),
                },
                {
                  key: 'remove',
                  label: t('canon.removeRelation'),
                  danger: true,
                  onSelect: () => unrelate.mutate(relation.link.id),
                },
              ]}
            />
          </div>
        ))}
        {relations.length === 0 && (
          <p className="px-2 text-xs text-faint">{t('canon.nothingHere')}</p>
        )}
        <Button
          size="xs"
          variant="ghost"
          className="mt-1 self-start"
          onClick={() => setRelating(true)}
        >
          <Plus aria-hidden />
          {t('canon.addRelation')}
        </Button>
        {relating && (
          <RelationDialog
            open
            onOpenChange={setRelating}
            card={view.card}
            kinds={section.kinds ?? []}
          />
        )}
        {changing !== null && (
          <RelationDialog
            key={changing.link.id}
            open
            onOpenChange={(open) => {
              if (!open) setChanging(null)
            }}
            card={view.card}
            kinds={section.kinds ?? []}
            relation={changing}
          />
        )}
      </>
    )
  } else if (shape === 'appearances') {
    inner = (
      <>
        {view.appearances.length === 0 ? (
          <p className="px-2 text-xs text-faint">{t('canon.appearsNowhere')}</p>
        ) : (
          <ul className="flex flex-col gap-0.5 px-2 text-sm">
            {view.appearances.map((one) => (
              <li key={one.work_id} className="flex flex-wrap items-center gap-1.5">
                <Button variant="link" onClick={() => void navigate(`/works/${one.work_id}`)}>
                  {one.title}
                </Button>
                <span className="text-xs text-faint">
                  {[
                    one.hero ? t('canon.appears.hero') : null,
                    one.scenes.length > 0
                      ? t('canon.appears.scenes', {
                          list: one.scenes.join(', '),
                          count: one.scenes.length,
                        })
                      : null,
                    one.named ? t('canon.appears.named') : null,
                    one.cover ? t('canon.appears.cover') : null,
                    one.facts > 0 ? t('canon.appears.facts', { count: one.facts }) : null,
                  ]
                    .filter((part) => part !== null)
                    .join(' · ')}
                </span>
              </li>
            ))}
          </ul>
        )}
        <p className="px-2 pt-1 text-xs text-faint">{t('canon.appearsHint')}</p>
      </>
    )
  } else {
    const every = facts ?? factsIn(view.facts, section.key)
    const day = today()
    const own =
      shape === 'bank' && standing !== 'all'
        ? every.filter((read) => bankState(read.fact.data, day).state === standing)
        : every
    const cut = own.length > LONG_SECTION && !all
    inner = (
      <>
        {shape === 'bank' && every.length > 0 && (
          <SegmentedControl
            aria-label={t('canon.bank.standing')}
            value={standing}
            onValueChange={(next) => setStanding(next as BankState | 'all')}
            className="mb-1 self-start"
          >
            {(['all', 'available', 'cooldown', 'locked'] as const).map((one) => (
              <Segment key={one} value={one}>
                {t(`canon.bank.filter.${one}`)}
              </Segment>
            ))}
          </SegmentedControl>
        )}
        {(cut ? own.slice(0, SHOWN_OF_LONG) : own).map((read) => (
          <FactRow key={read.fact.id} read={read} view={view} shape={shape} lens={lens} />
        ))}
        {cut && (
          <Button
            size="xs"
            variant="ghost"
            className="mt-1 self-start"
            onClick={() => setAll(true)}
          >
            {t('canon.showAllFacts', { count: own.length })}
          </Button>
        )}
        {own.length === 0 && !adding && (
          <p className="px-2 text-xs text-faint">{t('canon.nothingHere')}</p>
        )}
        {section.key !== '' &&
          (adding ? (
            <FactEditor
              view={view}
              section={section.key}
              shape={shape}
              onDone={() => setAdding(false)}
            />
          ) : (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setAdding(true)}
              className="mt-1 justify-start border border-dashed border-line-2 text-faint"
            >
              <Plus aria-hidden />
              {t(`canon.add.${shape}`)}
            </Button>
          ))}
      </>
    )
  }

  return (
    <section
      aria-label={sayLabel(section.label)}
      className={cn(
        'flex flex-col',
        panel ? 'rounded-lg border border-line bg-raise px-3.5 py-3' : 'pt-3',
      )}
    >
      {head}
      {!folded && inner}
      {checking && (
        <QuotaReportDialog
          open
          onOpenChange={setChecking}
          subject={{}}
          title={t('quota.titleSpace')}
        />
      )}
    </section>
  )
}

/** A layer as its letter, in the colour the canon gives it: A public, B
 *  internal, C only in the works. */
export function LayerMark({ layer }: { layer: 'public' | 'internal' | 'inWorks' }) {
  const { t } = useTranslation()
  // A card of public, settled facts shows no letters (v0.141).
  if (!useDetailed()) return null
  return (
    <span
      title={t(`canon.layer.long.${layer}`)}
      className={cn(
        'mt-0.5 grid h-5 w-5.5 shrink-0 place-items-center rounded-md font-mono text-2xs font-semibold',
        layer === 'public' && 'bg-good-soft text-good',
        layer === 'internal' && 'bg-warn-soft text-warn',
        layer === 'inWorks' && 'bg-accent-soft text-accent-2',
      )}
    >
      {t(`canon.layer.short.${layer}`)}
    </span>
  )
}
