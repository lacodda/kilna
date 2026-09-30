import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import { unrelateCards } from '@/lib/api/canon'
import type { CanonSection, CardView, ReadFact, Relation } from '@/lib/api/types'
import { factsIn, seenThrough, type LensChoice } from '@/lib/canon'
import { announceEdited } from '@/lib/edited'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { RowMenu } from '@/components/RowMenu'
import { FactEditor } from '@/features/canon/FactEditor'
import { FactRow } from '@/features/canon/FactRow'
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
}

/**
 * One section of a card, drawn by its shape: statements with the way to add
 * one, the card's relations with the way to draw one, or the works it
 * appears in, counted and never written.
 */
export function SectionBlock({ view, section, lens, onOpen, facts, panel = false }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [adding, setAdding] = useState(false)
  const [relating, setRelating] = useState(false)
  const [changing, setChanging] = useState<Relation | null>(null)
  const client = useQueryClient()
  const unrelate = useAppMutation({
    mutationFn: (id: string) => unrelateCards(id),
    failure: 'canon.relateFailed',
    onSuccess: () =>
      announceEdited({ client, message: t('canon.relationRemoved'), refresh: refresh.canon }),
  })
  const shape = section.shape ?? 'facts'
  const hint = section.hint !== undefined && section.hint !== null ? sayLabel(section.hint) : null

  const head = (
    <header className="mb-1 flex flex-wrap items-center gap-2">
      <b className={cn('font-semibold', panel ? 'text-base' : 'text-sm')}>
        {sayLabel(section.label)}
      </b>
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
    const own = facts ?? factsIn(view.facts, section.key)
    inner = (
      <>
        {own.map((read) => (
          <FactRow key={read.fact.id} read={read} view={view} shape={shape} lens={lens} />
        ))}
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
      {inner}
    </section>
  )
}

/** A layer as its letter, in the colour the canon gives it: A public, B
 *  internal, C only in the works. */
export function LayerMark({ layer }: { layer: 'public' | 'internal' | 'inWorks' }) {
  const { t } = useTranslation()
  return (
    <span
      title={t(`canon.layer.long.${layer}`)}
      className={cn(
        'mt-0.5 grid h-5 w-5.5 shrink-0 place-items-center rounded-md font-mono text-2xs font-bold',
        layer === 'public' && 'bg-good-soft text-good',
        layer === 'internal' && 'bg-warn-soft text-warn',
        layer === 'inWorks' && 'bg-accent-soft text-accent-2',
      )}
    >
      {t(`canon.layer.short.${layer}`)}
    </span>
  )
}
