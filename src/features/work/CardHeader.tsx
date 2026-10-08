import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { ArrowLeft, Pencil, Star } from 'lucide-react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { MetaField, ProfileConfig, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { useBlindJudging } from '@/lib/blindJudging'
import { useCardView } from '@/features/work/cardView'
import { coverImageFor } from '@/lib/cover'
import { fieldText } from '@/lib/fieldValue'
import { formatDay, formatTotal } from '@/lib/format'
import { useCovers } from '@/lib/useCovers'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceEdited } from '@/lib/edited'
import { badgeVariantOf } from '@/lib/markIcon'
import { wordOf } from '@/lib/overview'
import { say } from '@/lib/toast'
import {
  fieldsFor,
  hasDoors,
  labelOf,
  say as sayLabel,
  useProfile,
  vocabularyOf,
} from '@/lib/useProfile'
import { useStar } from '@/lib/useStar'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Copyable } from '@/components/ui/copyable'
import { CopyButton } from '@/components/ui/copy-button'
import { Input } from '@/components/ui/input'
import { RowMenu } from '@/components/RowMenu'
import { WorkRepeatMark } from '@/components/RepeatMark'
import { StagePicker } from '@/components/StagePicker'
import { CollectionPicker } from '@/features/collections/CollectionPicker'
import { WorkPlayer } from '@/features/work/player'
import { TabBar } from '@/features/work/TabBar'
import type { Tab, TabCount } from '@/features/work/tabs'
import { TagBar } from '@/features/work/TagBar'
import { useMakePublication } from '@/features/work/useMakePublication'

interface Props {
  work: Work
  /** The tabs this work draws, and the numbers beside them; the card knows both. */
  tabs: readonly Tab[]
  counts: Partial<Record<Tab, TabCount>>
  /** Deleting the work, from the header's menu. */
  onDelete: () => void
}

/**
 * The top of a work's card: what this is, at a glance, and the way between
 * its tabs.
 *
 * It stands still (the owner's call, 20.09). Until v0.75.2 the cover scrolled
 * away and the rest stuck to the top of a card that scrolled as one page;
 * now nothing here moves, and the open tab under it takes the rest of the
 * window and scrolls inside itself. The cover is a band rather than a banner
 * for the same reason: every pixel it takes, it takes from every tab.
 *
 * The cover is a gradient derived from the work's id (see `lib/cover`) until
 * real covers arrive; it is what makes one card distinguishable from another
 * before a single word is read.
 */
export function CardHeader({ work, tabs, counts, onDelete }: Props) {
  const { t } = useTranslation()
  const cover = useCovers().get(work.id)

  return (
    <header className="shrink-0 overflow-hidden rounded-t-xl border border-b-0 border-line bg-raise">
      {/* The cover: what tells one card from another at a glance, before a
          word is read. The way back sits on it because that is the one place
          on the card that carries nothing else. */}
      <div className="relative h-14" style={{ background: coverImageFor(work.id, cover) }}>
        <Link
          to="/catalogue"
          className="absolute top-2.25 left-3 inline-flex items-center gap-1.5 rounded-md bg-black/35 px-2.5 py-0.75 text-sm text-white/90 no-underline backdrop-blur-sm transition-colors hover:bg-black/50 hover:text-white/100"
        >
          <ArrowLeft aria-hidden className="size-3.5" />
          {t('nav.catalogue')}
        </Link>
      </div>

      {/* The name on a line of its own, and under it one row of chips: what
          the work is and where it stands, then what its author says about it
          this week and for good - the mockup's order. The kind and the
          verdict sat on the name's line until v0.80, where they read as part
          of the title. */}
      <div className="px-4 pt-2.25">
        <div className="flex flex-wrap items-center gap-x-2.25 gap-y-1.5">
          <Title work={work} />

          {/* Pushed to the end of the row: the actions are what you reach
              for, not what tells you whose card this is. */}
          <span className="ml-auto">
            <HeaderActions work={work} onDelete={onDelete} />
          </span>
        </div>

        <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
          <Standing work={work} />
          <TagBar work={work} />
        </div>

        <MetaStrip work={work} />
        <WorkPlayer workId={work.id} />
      </div>

      <TabBar workId={work.id} tabs={tabs} counts={counts} />
    </header>
  )
}

/**
 * What the work is and where it stands: its kind, its status, the verdict of
 * its last score, and the collection it belongs to. The first three are chips
 * that say rather than chips that do - each is changed where it is decided:
 * the status by what happens to the work, the verdict on the Score tab. The
 * collection is decided here as well, so its chip is a picker (v0.92).
 */
function Standing({ work }: { work: Work }) {
  const { t, i18n } = useTranslation()
  const profile = useProfile()
  const { hiding } = useBlindJudging()
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const doorless = !hasDoors(profile.config, work.kind)

  // The tier and total belong here rather than only on the Score tab: they are
  // the verdict, and the verdict is what someone opens a card to check.
  const score = useQuery(queries.latestScore(work.id))
  // A song's status is its publications' fact (ADR 0047), so the chip says
  // which one: asked only of a work that goes out through what is made from it.
  const publications = useQuery({ ...queries.publications(work.id), enabled: doorless })
  // And a publication says what it goes out for: the song is one click away
  // from its clip, its audio, its short - the way back the mockup draws.
  const links = useQuery({ ...queries.links(work.id), enabled: !doorless })
  const donor = links.data?.sources[0]

  const latest = score.data ?? null
  const status = vocabulary.statuses.find((s) => s.key === work.status)
  const said = status === undefined ? work.status : sayLabel(status.label)

  // The basis beside the word only when the word is that fact - "Released ·
  // as audio Jul 22". A status someone set by hand, "shelved", stands on no
  // publication even while one is out, and saying one beside it would read
  // as the reason for it.
  const basis = publications.data?.basis ?? null
  const standsOn =
    basis !== null && status?.derive === (basis.released ? 'released' : 'scheduled') ? basis : null

  return (
    <>
      <Chip>{labelOf(profile.config.work_kinds, work.kind)}</Chip>
      {choicesOf(profile.config, work).map(({ field, option }) => (
        <Chip key={field.key}>
          {t('work.choice', { field: sayLabel(field.label), value: option })}
        </Chip>
      ))}
      <Chip
        variant={badgeVariantOf(status?.colour)}
        title={
          standsOn === null
            ? undefined
            : t(standsOn.released ? 'publications.outAs' : 'publications.bookedAs', {
                title: standsOn.title,
                day: formatDay(standsOn.day),
              })
        }
      >
        {standsOn === null
          ? said
          : t('work.standsOn', {
              status: said,
              kind: wordOf(labelOf(profile.config.work_kinds, standsOn.kind), i18n.language),
              day: formatDay(standsOn.day),
            })}
      </Chip>

      {/* The guard's mark beside where the work stands (ADR 0054): a song
          its own, a publication its song's - a clip repeats what its song
          says. The findings themselves are on the overview. */}
      <WorkRepeatMark workId={work.id} look="badge" />

      {donor !== undefined && (
        <Link
          to={`/works/${donor.source_id}`}
          className="truncate text-xs text-dim hover:text-text hover:underline"
        >
          {t('work.madeFrom', { title: donor.source_title })}
        </Link>
      )}

      {/* Held back while this card is judged blind: it is the verdict the
          mode exists to hide, one line above the scales. */}
      {latest !== null && !hiding && (
        <Chip variant="accent">
          {latest.tier !== null && `${labelOf(vocabulary.tiers, latest.tier)} · `}
          <span className="font-mono tabular-nums">{formatTotal(latest.total)}</span>
        </Chip>
      )}

      <CollectionPicker work={work} />
    </>
  )
}

/**
 * The choice fields of the work's kind that hold an answer - the variant of
 * an audio release: what kind of thing this one is, said beside the kind,
 * by the answer's label rather than its key.
 */
function choicesOf(config: ProfileConfig, work: Work): { field: MetaField; option: string }[] {
  return fieldsFor(config, work.kind)
    .filter((field) => field.type === 'choice')
    .flatMap((field) => {
      const option = fieldText(field, work.meta[field.key])
      return option === null ? [] : [{ field, option }]
    })
}

/**
 * The name, and the three things done to it: rename in place, copy it, copy
 * the id — with the star beside them.
 *
 * Renaming turns the heading into a box of the same height, so the header
 * does not move under the cursor: Enter saves, Escape cancels, and
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

  const rename = useAppMutation({
    mutationFn: (next: string) => updateWork(work.id, { title: next }),
    failure: 'toast.workSaveFailed',
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      announceEdited({
        client,
        message: t('toast.workRenamed'),
        refresh: [keys.works, keys.catalogue, keys.journal],
      })
    },
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
  // Both copies below draw and announce their own tick; only a refusal is
  // raised here, as a toast. The copy says that the clipboard said no, not
  // why, so the toast carries the fact alone.
  const refused = (ok: boolean) => {
    if (!ok) say.failed(t('work.copyFailed'))
  }

  // `group`: the title's copy button shows while the name's row is pointed at.
  return (
    <span className="group inline-flex min-w-0 items-center gap-1">
      {draft === null ? (
        <h1 className="truncate text-xl font-[650] tracking-tight">{work.title}</h1>
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
          className="w-80 max-w-full text-lg font-[650] tracking-tight"
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
          <CopyButton
            value={work.title}
            label={t('work.copyTitle')}
            copiedLabel={t('work.copied')}
            title={t('work.copyTitle')}
            onCopy={refused}
          />
          <Copyable
            value={work.id}
            label={t('work.copyId')}
            copiedLabel={t('work.copied')}
            title={t('work.copyIdHint')}
            onCopy={refused}
          >
            {work.id.slice(0, 8)}
          </Copyable>
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
 * copy a link to the card, make another kind of work from this one, delete it.
 *
 * Delete lived in a footer under the open tab, which was the bottom of a card
 * that scrolled as one page. The card no longer scrolls, and a button at the
 * foot of whichever tab is open is a button that is somewhere else on every
 * tab. The deletion is undoable from its toast, as it always was.
 */
function HeaderActions({ work, onDelete }: { work: Work; onDelete: () => void }) {
  const { t } = useTranslation()

  // A work of another kind made from this one — a clip from a song. One entry
  // per kind that can be made, so the menu says what can be made rather than
  // opening a dialog to ask; the same gesture as the overview's Make menu.
  const make = useMakePublication(work)

  // The tick only after the clipboard confirms — the rule from v0.28: telling
  // someone a copy succeeded when it did not is worse than saying nothing.
  const copy = (value: string) => {
    navigator.clipboard.writeText(value).then(
      () => say.ok(t('work.copied')),
      (cause: unknown) => say.failedTo(t('work.copyFailed'), cause),
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
        ...make.kinds.map((kind) => ({
          key: `derive:${kind.key}`,
          label: kind.label,
          onSelect: () => make.make(kind.key),
        })),
        { key: 'delete', label: t('work.delete'), onSelect: onDelete, danger: true },
      ]}
    />
  )
}

function MetaStrip({ work }: { work: Work }) {
  const profile = useProfile()
  const { view } = useCardView()

  // Off unless this machine asked for it. Switched rather than deleted: the
  // strip is the fastest way to compare a tempo against a mood for whoever
  // works that way, and the fields themselves are untouched either way.
  if (!view.metaStrip) return null

  // The fields of this work's kind: the variant of an audio release is no
  // field of a song's, even when an old value is still in its meta. A choice
  // reads as its answer's label, not the key it is stored by (`fieldText`).
  const filled = fieldsFor(profile.config, work.kind)
    // A paragraph belongs on the Overview tab, not here. The strip is the
    // reference line you glance at — a premise printed in full took half the
    // screen above the tabs and pushed the work out of sight, which is the
    // opposite of what a header carrying the title is for.
    .filter((field) => field.type !== 'multiline')
    .flatMap((field) => {
      const text = fieldText(field, work.meta[field.key])
      return text === null ? [] : [{ field, text }]
    })
  if (filled.length === 0) return null

  return (
    <div className="mt-2.5 flex flex-wrap gap-5.5">
      {filled.map(({ field, text }) => {
        return (
          // Each field is capped in width and truncated. A craft writes what it
          // likes into these — a mood can be a sentence, a vocal note a whole
          // line — and seven of them at full length grew the header to 387px,
          // pushing the work itself off screen. The full value is a hover away
          // and edited on the Overview tab.
          <span key={field.key} className="min-w-0 max-w-56" title={text}>
            <label className="block caption">{sayLabel(field.label)}</label>
            <b className="block truncate font-mono text-sm font-medium">{text}</b>
          </span>
        )
      })}
    </div>
  )
}
