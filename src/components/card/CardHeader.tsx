import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { ArrowLeft } from 'lucide-react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { latestScore, listCollections, updateWork, type Work } from '@/lib/api'
import { coverFor } from '@/lib/cover'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { Dialog } from '@/components/ui/Dialog'
import { Field, Input } from '@/components/ui/Input'
import { RowMenu } from '@/components/ui/RowMenu'
import { TabBar } from '@/components/card/TabBar'
import { TagBar } from '@/components/card/TagBar'

interface Props {
  work: Work
  /** Shown on the Releases tab; the card already knows the count. */
  releases: number
}

/**
 * The top of a work's card: what this is, at a glance, wherever you have
 * scrolled to.
 *
 * The cover is a gradient derived from the work's id (see `lib/cover`) until
 * real covers arrive; it is what makes one card distinguishable from another
 * before a single word is read.
 */
export function CardHeader({ work, releases }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  // The tier and total belong here rather than only on the Score tab: they are
  // the verdict, and the verdict is what someone opens a card to check.
  const score = useQuery({
    queryKey: keys.latestScore(work.id),
    queryFn: () => latestScore(work.id),
  })
  const collections = useQuery({
    queryKey: keys.collections,
    queryFn: listCollections,
    // Only fetched when the work is actually in one.
    enabled: work.collection_id !== null,
  })

  const collection = collections.data?.find((item) => item.id === work.collection_id)
  const latest = score.data ?? null

  // Renaming opens a dialog rather than turning the heading into a field. The
  // header is sticky, and a box that saves on blur inside it moves under the
  // cursor as it does — the same reason the meta values here are read-only.
  const [renaming, setRenaming] = useState(false)

  // Two siblings rather than one header, because a sticky element can never
  // leave its own parent's box: wrapped together, the bar would unstick the
  // moment the (short) header scrolled past, which is exactly when it is needed.
  // As siblings of the tab body, both are bounded by the whole scrolling column.
  return (
    <>
      {/* The cover. It scrolls away: it is what tells one card from another at a
          glance, not something to navigate by. The way back sits on it because
          that is the one place on the card that carries nothing else — and on a
          narrow window the list beside it is gone, leaving the browser's back
          button as the only way out. */}
      <div className="rounded-t-[18px] border border-b-0 border-line">
        <div
          className="relative h-[118px] rounded-t-[17px]"
          style={{ background: coverFor(work.id) }}
        >
          <Link
            to="/catalogue"
            className="absolute left-3.5 top-3.5 inline-flex items-center gap-1.5 rounded-[10px] bg-black/35 px-2.5 py-1 text-[13px] text-white/90 backdrop-blur-sm transition-colors hover:bg-black/50 hover:text-white"
          >
            <ArrowLeft aria-hidden className="size-3.5" />
            {t('nav.catalogue')}
          </Link>
        </div>
      </div>

      {/* What stays: whose card this is, where it stands, and the way between
          tabs. A long version otherwise leaves you reading with no idea whose
          words they are. `top-0` is relative to the scrolling screen area. */}
      <header className="sticky -top-px z-20 overflow-hidden rounded-b-[18px] border border-t-0 border-line bg-raise">
        {/* Name first, then what it is, then the craft's own numbers — the
            order of the mockup, and the order someone reads in: the title says
            whose card this is, the badges where it stands, and BPM/Key are
            reference you consult rather than identify by. They sat above the
            title until v0.20, which read as though the numbers were the
            heading. Read-only here; they are edited on the Overview tab, and a
            header that can be typed into shifts under the cursor as it saves. */}
        <div className="px-[18px] pt-3 pb-2.5">
          <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1.5">
            <h2 className="text-[21px] font-[650] tracking-[-0.01em]">{work.title}</h2>

            <Badge>{labelOf(profile.config.work_kinds, work.kind)}</Badge>
            <Badge variant="accent">{labelOf(profile.config.statuses, work.status)}</Badge>

            {latest !== null && (
              <Badge variant="soft">
                {latest.tier !== null && `${labelOf(profile.config.tiers, latest.tier)} · `}
                <span className="font-mono tabular-nums">{Math.round(latest.total * 10) / 10}</span>
              </Badge>
            )}

            {collection !== undefined && <Badge>{collection.title}</Badge>}

            {/* Pushed to the end of the row: the actions are what you reach
                for, not what tells you whose card this is. */}
            <span className="ml-auto">
              <HeaderActions work={work} onRename={() => setRenaming(true)} />
            </span>
          </div>

          <TagBar work={work} />

          <MetaStrip work={work} />
        </div>

        <TabBar workId={work.id} releases={releases} />
      </header>

      <RenameDialog work={work} open={renaming} onOpenChange={setRenaming} />
    </>
  )
}

/**
 * What you can do to the work from its header.
 *
 * Copying is here rather than on the Overview tab because it is what you do
 * *with* a card, not to it: quoting the title in a message, pasting the id into
 * a script, sending someone the card itself.
 */
function HeaderActions({ work, onRename }: { work: Work; onRename: () => void }) {
  const { t } = useTranslation()

  // The tick only after the clipboard confirms — the rule from v0.28: telling
  // someone a copy succeeded when it did not is worse than saying nothing.
  const copy = (value: string) => {
    navigator.clipboard.writeText(value).then(
      () => say.ok(t('work.copied')),
      (cause: unknown) => say.failedTo(t('work.copied'), cause),
    )
  }

  return (
    <RowMenu
      label={work.title}
      actions={[
        { key: 'rename', label: t('work.rename'), onSelect: onRename },
        { key: 'title', label: t('work.copyTitle'), onSelect: () => copy(work.title) },
        { key: 'id', label: t('work.copyId'), onSelect: () => copy(work.id) },
        {
          key: 'link',
          label: t('work.copyLink'),
          // The address of the card inside the app: routing is real (ADR 0006),
          // so this is a link that opens the work rather than a note of where
          // it lives.
          onSelect: () => copy(`kilna://works/${work.id}`),
        },
      ]}
    />
  )
}

function RenameDialog({
  work,
  open,
  onOpenChange,
}: {
  work: Work
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [title, setTitle] = useState(work.title)

  // The box starts from what the work is called each time it opens, not from
  // whatever was typed and abandoned last time.
  const [syncedTo, setSyncedTo] = useState(work.title)
  if (open && syncedTo !== work.title) {
    setSyncedTo(work.title)
    setTitle(work.title)
  }

  const rename = useMutation({
    mutationFn: (next: string) => updateWork(work.id, { title: next }),
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.journal })
      onOpenChange(false)
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const submit = () => {
    const next = title.trim()
    // An empty box or an unchanged name is a cancel, not an error: nothing was
    // asked for, so nothing is said about it.
    if (next === '' || next === work.title) return onOpenChange(false)
    rename.mutate(next)
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('work.rename')}
      footer={
        <Button variant="primary" disabled={rename.isPending} onClick={submit}>
          {t('work.rename')}
        </Button>
      }
    >
      <Field label={t('work.title')} hint={t('work.renameHint')}>
        <Input
          autoFocus
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') submit()
          }}
        />
      </Field>
    </Dialog>
  )
}

function MetaStrip({ work }: { work: Work }) {
  const { i18n } = useTranslation()
  const profile = useProfile()

  const filled = profile.config.work_meta_fields.filter(
    (field) =>
      // A paragraph belongs on the Overview tab, not here. The strip is the
      // reference line you glance at — a premise printed in full took half the
      // screen above the tabs and pushed the work out of sight, which is the
      // opposite of what a header carrying the title is for.
      field.type !== 'multiline' &&
      work.meta[field.key] !== undefined &&
      work.meta[field.key] !== '',
  )
  if (filled.length === 0) return null

  return (
    <div className="mt-2.5 flex flex-wrap gap-[22px]">
      {filled.map((field) => {
        const value = work.meta[field.key]
        const text =
          field.type === 'boolean'
            ? i18n.t(value === true ? 'work.yes' : 'work.no')
            : String(value)
        return (
          // Each field is capped in width and truncated. A craft writes what it
          // likes into these — a mood can be a sentence, a vocal note a whole
          // line — and seven of them at full length grew the header to 387px,
          // pushing the work itself off screen. The full value is a hover away
          // and edited on the Overview tab.
          <span key={field.key} className="min-w-0 max-w-56" title={text}>
            <label className="block text-[10px] uppercase tracking-[0.08em] text-faint">
              {field.label}
            </label>
            <b className="block truncate font-mono text-[12.5px] font-medium">{text}</b>
          </span>
        )
      })}
    </div>
  )
}
