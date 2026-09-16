import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { ArrowLeft, Copy, Pencil, Star } from 'lucide-react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { deriveWork, latestScore, listCollections, updateWork, type Work } from '@/lib/api'
import { useCardView } from '@/lib/cardView'
import { coverImageFor } from '@/lib/cover'
import { useCovers } from '@/lib/useCovers'
import { keys } from '@/lib/query'
import { announceEdited } from '@/lib/edited'
import { badgeVariantOf } from '@/lib/markIcon'
import { say } from '@/lib/toast'
import { labelOf, useProfile, vocabularyOf } from '@/lib/useProfile'
import { useStar } from '@/lib/useStar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { RowMenu } from '@/components/ui/RowMenu'
import { StagePicker } from '@/components/StagePicker'
import { TabBar } from '@/components/card/TabBar'
import { TagBar } from '@/components/card/TagBar'

interface Props {
  work: Work
  /** Shown on the Releases tab; the card already knows the count. */
  releases: number
  /** Shown on the Links tab: sources and works made from this. */
  links?: number
  /** Undefined for a kind with no storyboard: the tab is then not drawn. */
  scenes?: number
}

/**
 * The top of a work's card: what this is, at a glance, wherever you have
 * scrolled to.
 *
 * The cover is a gradient derived from the work's id (see `lib/cover`) until
 * real covers arrive; it is what makes one card distinguishable from another
 * before a single word is read.
 */
export function CardHeader({ work, releases, links = 0, scenes }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const cover = useCovers().get(work.id)

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
  const status = vocabulary.statuses.find((s) => s.key === work.status)

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
          style={{ background: coverImageFor(work.id, cover) }}
        >
          <Link
            to="/catalogue"
            className="absolute left-3.5 top-3.5 inline-flex items-center gap-1.5 rounded-[10px] bg-black/35 px-2.5 py-1 text-[13px] text-white/90 backdrop-blur-sm transition-colors hover:bg-black/50 hover:text-white/100"
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
            <Title work={work} />

            <Badge>{labelOf(profile.config.work_kinds, work.kind)}</Badge>
            <Badge variant={badgeVariantOf(status?.colour)}>{status?.label ?? work.status}</Badge>

            {latest !== null && (
              <Badge variant="soft">
                {latest.tier !== null && `${labelOf(vocabulary.tiers, latest.tier)} · `}
                <span className="font-mono tabular-nums">{Math.round(latest.total * 10) / 10}</span>
              </Badge>
            )}

            {collection !== undefined && <Badge>{collection.title}</Badge>}

            {/* Pushed to the end of the row: the actions are what you reach
                for, not what tells you whose card this is. */}
            <span className="ml-auto">
              <HeaderActions work={work} />
            </span>
          </div>

          <TagBar work={work} />

          <MetaStrip work={work} />
        </div>

        <TabBar workId={work.id} releases={releases} links={links} scenes={scenes} />
      </header>
    </>
  )
}

/**
 * The name, and the three things done to it: rename in place, copy it, copy
 * the id — with the star beside them.
 *
 * Renaming turns the heading into a box of the same height, so the sticky
 * header does not move under the cursor: Enter saves, Escape cancels, and
 * leaving the box saves what was typed — the same bargain as every field on
 * the Scenes tab. The dialog it replaces was one click and one dialog more
 * than a name deserves. The id is copied by clicking it: it is here to be
 * pasted into a script or told to an agent, not read.
 */
function Title({ work }: { work: Work }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [draft, setDraft] = useState<string | null>(null)
  const star = useStar(work.id)
  const starred = work.bookmarked_at !== null

  const rename = useMutation({
    mutationFn: (next: string) => updateWork(work.id, { title: next }),
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      announceEdited({
        client,
        message: t('toast.workRenamed'),
        refresh: [keys.works, keys.catalogue, keys.journal],
      })
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const commit = () => {
    const next = (draft ?? '').trim()
    setDraft(null)
    // An empty box or an unchanged name is a cancel, not an error: nothing was
    // asked for, so nothing is said about it.
    if (next === '' || next === work.title) return
    rename.mutate(next)
  }

  // The tick only after the clipboard confirms — the rule from v0.28: telling
  // someone a copy succeeded when it did not is worse than saying nothing.
  const copy = (value: string) => {
    navigator.clipboard.writeText(value).then(
      () => say.ok(t('work.copied')),
      (cause: unknown) => say.failedTo(t('work.copied'), cause),
    )
  }

  return (
    <span className="inline-flex min-w-0 items-center gap-1">
      {draft === null ? (
        <h2 className="truncate text-[21px] font-[650] tracking-[-0.01em]">{work.title}</h2>
      ) : (
        <Input
          autoFocus
          value={draft}
          aria-label={t('work.title')}
          title={t('work.renameHint')}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === 'Enter') commit()
            if (event.key === 'Escape') setDraft(null)
          }}
          className="h-8 w-80 max-w-full text-[17px] font-[650] tracking-[-0.01em]"
        />
      )}
      {draft === null && (
        <>
          <Button
            variant="icon"
            size="icon-sm"
            title={t('work.rename')}
            aria-label={t('work.rename')}
            onClick={() => setDraft(work.title)}
          >
            <Pencil aria-hidden />
          </Button>
          <Button
            variant="icon"
            size="icon-sm"
            title={t('work.copyTitle')}
            aria-label={t('work.copyTitle')}
            onClick={() => copy(work.title)}
          >
            <Copy aria-hidden />
          </Button>
          <button
            type="button"
            title={t('work.copyIdHint')}
            onClick={() => copy(work.id)}
            className="cursor-pointer rounded px-1 font-mono text-[11.5px] text-faint transition-colors hover:text-dim"
          >
            {work.id.slice(0, 8)}
          </button>
          <Button
            variant="icon"
            size="icon-sm"
            aria-pressed={starred}
            title={t(starred ? 'work.unstar' : 'work.star')}
            aria-label={t(starred ? 'work.unstar' : 'work.star')}
            disabled={star.isPending}
            onClick={() => star.mutate(!starred)}
            className={starred ? 'text-warn hover:text-warn' : undefined}
          >
            <Star aria-hidden className={starred ? 'fill-current' : undefined} />
          </Button>

          {/* Beside the star, because the two are the same kind of thing: what
              the author says about the work by hand, as against everything
              else on this header, which is derived from what happened. */}
          <StagePicker workId={work.id} percent={work.stage} />
        </>
      )}
    </span>
  )
}

/**
 * What you can do to the work from its header, beyond the name's own row:
 * copy a link to the card, make another kind of work from this one.
 */
function HeaderActions({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const navigate = useNavigate()

  // A work of another kind made from this one — a video from a song. One
  // entry per other kind of the profile, so the menu says what can be made
  // rather than opening a dialog to ask.
  const derive = useMutation({
    mutationFn: (kind: string) => deriveWork(work.id, kind),
    onSuccess: (created) => {
      for (const key of [keys.works, keys.catalogue, keys.links, keys.journal]) {
        void client.invalidateQueries({ queryKey: key })
      }
      say.ok(t('links.made', { title: created.title }))
      void navigate(`/works/${created.id}/links`)
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

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
        {
          key: 'link',
          label: t('work.copyLink'),
          // The address of the card inside the app: routing is real (ADR 0006),
          // so this is a link that opens the work rather than a note of where
          // it lives.
          onSelect: () => copy(`kilna://works/${work.id}`),
        },
        ...profile.config.work_kinds
          .filter((kind) => kind.key !== work.kind)
          .map((kind) => ({
            key: `derive:${kind.key}`,
            label: t('links.makeFromThis', { kind: kind.label }),
            onSelect: () => derive.mutate(kind.key),
          })),
      ]}
    />
  )
}

function MetaStrip({ work }: { work: Work }) {
  const { i18n } = useTranslation()
  const profile = useProfile()
  const { view } = useCardView()

  // Off unless this machine asked for it. Switched rather than deleted: the
  // strip is the fastest way to compare a tempo against a mood for whoever
  // works that way, and the fields themselves are untouched either way.
  if (!view.metaStrip) return null

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
