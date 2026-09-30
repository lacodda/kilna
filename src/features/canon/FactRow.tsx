import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open as openFile } from '@tauri-apps/plugin-dialog'
import {
  deleteFact,
  expandCardReferences,
  reorderFacts,
  retireFact,
  setPictureRole,
  updateFact,
} from '@/lib/api/canon'
import { attachAsset, detachAsset } from '@/lib/api/assets'
import type { CardView, FactPatch, ReadFact, SectionShape } from '@/lib/api/types'
import { PICTURES } from '@/lib/media'
import { factsIn, seenThrough, whenOf, type LensChoice } from '@/lib/canon'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { CardPicture } from '@/features/canon/CardPicture'
import { FactEditor } from '@/features/canon/FactEditor'
import { LayerMark } from '@/features/canon/SectionBlock'
import { cn } from '@/lib/utils'

interface Props {
  read: ReadFact
  view: CardView
  shape: SectionShape
  lens: LensChoice
}

/**
 * One fact: its layer as a letter, its words - or, for a section of another
 * shape, the slot, the colour, the template, the code - and beside them when
 * it happened, how settled it is and where it came from. Dimmed when the task
 * looked through cannot see it; struck through when it was retired, with the
 * reason beside it.
 */
export function FactRow({ read, view, shape, lens }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [editing, setEditing] = useState(false)
  const [retiring, setRetiring] = useState(false)
  const [reason, setReason] = useState('')
  const fact = read.fact

  const change = useAppMutation({
    mutationFn: (patch: FactPatch) => updateFact(fact.id, patch),
    failure: 'toast.factSaveFailed',
    refresh: refresh.canon,
  })
  const retire = useAppMutation({
    mutationFn: () => retireFact(fact.id, reason.trim()),
    failure: 'toast.factSaveFailed',
    refresh: refresh.canon,
    onSuccess: () => {
      setRetiring(false)
      setReason('')
    },
  })
  // The facts of the section in their order: moving one is a new order for
  // all of them, the way the backend takes it.
  const siblings = factsIn(view.facts, fact.section).map((one) => one.fact.id)
  const at = siblings.indexOf(fact.id)
  const move = useAppMutation({
    mutationFn: (by: -1 | 1) => {
      const order = [...siblings]
      order.splice(at, 1)
      order.splice(at + by, 0, fact.id)
      return reorderFacts(fact.note_id, fact.section, order)
    },
    failure: 'toast.factSaveFailed',
    refresh: refresh.canon,
  })
  // A picture of the fact itself: an outfit's photo, a variant of the mark in
  // its files. A mark is drawn as a vector as often as not, so it takes SVG.
  const pictureRole =
    shape === 'marks' ? 'mark' : fact.section === 'outfits' ? 'outfit' : 'reference'
  const attach = useAppMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths)
        await attachAsset(path, { canon_fact_id: fact.id, kind: pictureRole })
    },
    failure: 'canon.pictureFailed',
    refresh: refresh.canon,
  })
  const role = useAppMutation({
    mutationFn: ({ id, next }: { id: string; next: string }) => setPictureRole(id, next),
    failure: 'canon.pictureFailed',
    refresh: refresh.canon,
  })
  const detach = useAppMutation({
    mutationFn: (id: string) => detachAsset(id),
    refresh: refresh.canon,
  })
  const choosePictures = async () => {
    const chosen = await openFile({
      multiple: true,
      title: t('canon.addPicture'),
      filters: [
        {
          name: t('styles.pictures'),
          extensions: shape === 'marks' ? [...PICTURES, 'svg'] : [...PICTURES],
        },
      ],
    })
    const paths = chosen === null ? [] : Array.isArray(chosen) ? chosen : [chosen]
    if (paths.length > 0) attach.mutate(paths.filter((path) => typeof path === 'string'))
  }

  const remove = useAppMutation({
    mutationFn: () => deleteFact(fact.id),
    failure: 'toast.factSaveFailed',
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('canon.factDeleted'),
        refresh: refresh.canon,
      }),
  })

  if (editing) {
    return (
      <FactEditor
        view={view}
        section={fact.section}
        shape={shape}
        fact={fact}
        onDone={() => setEditing(false)}
      />
    )
  }

  // A detail's template as a generator is handed it: every card it names
  // given by its description, which is what a picture needs.
  const template = typeof fact.data.template === 'string' ? fact.data.template : ''
  const copyTemplate = async () => {
    try {
      await navigator.clipboard.writeText(await expandCardReferences(template))
      say.ok(t('canon.promptCopied'))
    } catch (cause) {
      say.failedTo(t('canon.copyFailed'), cause)
    }
  }

  const retired = fact.status === 'retired'
  const actions: RowAction[] = [
    { key: 'edit', label: t('canon.editFact'), onSelect: () => setEditing(true) },
    ...(shape === 'details' && template.trim() !== ''
      ? [
          {
            key: 'copy',
            label: t('canon.copyForGenerator'),
            onSelect: () => void copyTemplate(),
          },
        ]
      : []),
    ...(fact.status === 'draft' || fact.status === 'open'
      ? [
          {
            key: 'settle',
            label: t('canon.settle'),
            onSelect: () => change.mutate({ status: 'canon' }),
          },
        ]
      : []),
    retired
      ? {
          key: 'restore',
          label: t('canon.unretire'),
          onSelect: () => change.mutate({ status: 'canon', retired_reason: null }),
        }
      : { key: 'retire', label: t('canon.retire'), onSelect: () => setRetiring(true) },
    {
      key: 'picture',
      label: t('canon.attachToFact'),
      onSelect: () => void choosePictures(),
    },
    ...(at > 0 ? [{ key: 'up', label: t('canon.moveUp'), onSelect: () => move.mutate(-1) }] : []),
    ...(at >= 0 && at < siblings.length - 1
      ? [{ key: 'down', label: t('canon.moveDown'), onSelect: () => move.mutate(1) }]
      : []),
    { key: 'delete', label: t('canon.deleteFact'), danger: true, onSelect: () => remove.mutate() },
  ]

  const when = whenOf(fact)
  const source = fact.source
  const pictures = view.pictures.filter((picture) => picture.canon_fact_id === fact.id)

  return (
    <div
      className={cn(
        'grid grid-cols-[24px_minmax(0,1fr)_auto] items-start gap-2.5 rounded-md px-2 py-1.5 transition-opacity hover:bg-soft/50',
        !seenThrough(read.lenses, lens) && 'opacity-25',
        fact.status === 'draft' && 'bg-info-soft/60',
      )}
    >
      <LayerMark layer={fact.layer} />
      <div className="min-w-0">
        <div
          className={cn('selectable text-sm leading-relaxed', retired && 'text-faint line-through')}
        >
          <FactWords fact={fact} shape={shape} />
        </div>
        {pictures.length > 0 && (
          <div className="mt-1 flex flex-wrap gap-1">
            {pictures.map((picture) => (
              <CardPicture
                key={picture.id}
                picture={picture}
                small
                onRole={(next) => role.mutate({ id: picture.id, next })}
                onRemove={() => detach.mutate(picture.id)}
              />
            ))}
          </div>
        )}
        <div className="mt-0.5 flex flex-wrap items-center gap-1.5">
          {when !== null && <span className="font-mono text-2xs text-dim">{when}</span>}
          <span
            className={cn(
              'rounded-sm px-1 text-2xs',
              fact.status === 'canon' && 'bg-good-soft text-good',
              (fact.status === 'draft' || fact.status === 'open') && 'bg-info-soft text-info',
              retired && 'bg-soft text-faint',
            )}
          >
            {t(`canon.status.${fact.status}`)}
          </span>
          {source !== null && source !== undefined && (
            <span
              className="rounded-sm border border-line px-1 text-2xs text-dim"
              title={source.line ?? undefined}
            >
              {source.kind === 'work' && source.work_id ? (
                <Button
                  variant="link"
                  className="text-2xs"
                  onClick={() => void navigate(`/works/${source.work_id ?? ''}`)}
                >
                  {source.label ?? t('canon.aWork')}
                </Button>
              ) : (
                source.label
              )}
              {source.line ? ` · «${source.line}»` : ''}
            </span>
          )}
          {fact.scope_work_id !== null && (
            <span className="rounded-sm border border-line px-1 text-2xs text-dim">
              {t('canon.onlyForOneWork')}
            </span>
          )}
          {retired && fact.retired_reason !== null && (
            <span className="text-2xs text-faint">{fact.retired_reason}</span>
          )}
        </div>
      </div>
      <RowMenu actions={actions} label={t('canon.factActions')} />

      <Dialog
        open={retiring}
        onOpenChange={setRetiring}
        title={t('canon.retireTitle')}
        description={t('canon.retireBody')}
        footer={
          <Button
            variant="primary"
            disabled={reason.trim() === '' || retire.isPending}
            onClick={() => retire.mutate()}
          >
            {t('canon.retire')}
          </Button>
        }
      >
        <Field label={t('canon.retireReason')}>
          <Input
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && reason.trim() !== '') retire.mutate()
            }}
          />
        </Field>
      </Dialog>
    </div>
  )
}

/** What a fact says, in the shape of its section. */
function FactWords({ fact, shape }: { fact: ReadFact['fact']; shape: SectionShape }) {
  const { t } = useTranslation()
  const text = (key: string) => {
    const value = fact.data[key]
    return typeof value === 'string' ? value : ''
  }
  const bricks = useQuery({ ...queries.styleBricksMatching(null, ''), enabled: shape === 'styles' })

  switch (shape) {
    case 'slots':
      return (
        <>
          <span className="mr-1.5 rounded-sm bg-info-soft px-1 font-mono text-2xs text-info">{`{${text('slot')}}`}</span>
          {fact.body}
        </>
      )
    case 'palette':
      return (
        <span className="flex items-center gap-2">
          <span
            aria-hidden
            className="size-5 rounded-md shadow-[inset_0_0_0_1px_var(--color-line)]"
            style={{ background: text('color') }}
          />
          {fact.body}
          <span className="font-mono text-2xs text-faint">{text('color')}</span>
        </span>
      )
    case 'details': {
      const on = fact.data.on !== false
      const places = Array.isArray(fact.data.places) ? (fact.data.places as string[]) : []
      return (
        <>
          <span className="flex flex-wrap items-center gap-1.5">
            <b className="font-semibold">{fact.body}</b>
            <span
              className={cn(
                'rounded-sm px-1 text-2xs',
                on ? 'bg-accent-soft text-accent-2' : 'bg-soft text-faint',
              )}
            >
              {on ? t('canon.detailOn') : t('canon.detailOff')}
            </span>
            <span className="text-2xs text-faint">
              {places.map((place) => t(`canon.place.${place}`)).join(' · ')}
            </span>
          </span>
          <span className="block font-mono text-2xs leading-normal text-dim">
            {text('template')}
          </span>
        </>
      )
    }
    case 'marks':
      return (
        <>
          <span className="flex flex-wrap items-center gap-1.5">
            {text('code') !== '' && (
              <span className="font-mono text-xs font-bold">{text('code')}</span>
            )}
            <b className="font-semibold">{fact.body}</b>
          </span>
          {text('prompt') !== '' && (
            <span className="block font-mono text-2xs leading-normal text-dim">
              {text('prompt')}
            </span>
          )}
        </>
      )
    case 'styles': {
      const brick = (bricks.data ?? []).find((one) => one.id === text('styleId'))
      return (
        <>
          <b className="font-semibold">{brick?.name ?? t('canon.styleGone')}</b>
          {fact.body !== '' && fact.body !== brick?.name && (
            <span className="text-dim"> — {fact.body}</span>
          )}
        </>
      )
    }
    default:
      return <>{fact.body}</>
  }
}
