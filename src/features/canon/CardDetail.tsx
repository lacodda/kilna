import { useCallback, useMemo, useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import {
  ArrowUpRight,
  ChevronDown,
  ChevronRight,
  ListPlus,
  Pencil,
  Plus,
  Sparkles,
} from 'lucide-react'
import { startCardTask } from '@/lib/api/canon'
import { deleteNote, updateNote } from '@/lib/api/notes'
import type { CardView, Layer, NotePatch } from '@/lib/api/types'
import { cardAction } from '@/lib/actions'
import { cardKindOf, factsIn, LAYERS, type LensChoice } from '@/lib/canon'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { useAssistant } from '@/lib/useAssistant'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { useFoldedAll } from '@/lib/folded'
import { Input } from '@/components/ui/input'
import { SaveState } from '@/components/ui/save-state'
import { Select } from '@/components/AppSelect'
import { Markdown } from '@/components/Markdown'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { Pane } from '@/components/ui/frame'
import { CardAvatar } from '@/features/canon/CardAvatar'
import { ChannelView } from '@/features/canon/ChannelView'
import { PromptBox } from '@/features/canon/PromptBox'
import { ProposalsBanner } from '@/features/canon/ProposalsBanner'
import { Detailed, isDetailed } from '@/features/canon/disclosure'
import { SectionBlock } from '@/features/canon/SectionBlock'
import { NoteBodyEditor } from '@/features/notes/NoteBodyEditor'
import { useNoteBody } from '@/features/notes/useNoteBody'

interface Props {
  view: CardView
  lens: LensChoice
  onOpen: (id: string) => void
  /** The card left: deleted. */
  onGone: () => void
}

/** A card holding this many facts offers a search inside it (v0.142). */
const SEARCHABLE = 10

/**
 * One card, open: its face and names along the top, a row of its sections to
 * jump to, and the sections themselves - facts with their layer, state,
 * source and time; the relations; where it appears - and last its free note,
 * the note the card was before it had facts.
 *
 * The channel, the root card, is laid out as its own board of typed panels
 * (`ChannelView`); every other kind reads down this column.
 */
export function CardDetail({ view, lens, onOpen, onGone }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const assistant = useAssistant()
  const { config } = useProfile()
  const card = view.card
  const kind = cardKindOf(config, card.kind)
  // Sections opened from "+ Add section" (v0.141), shown though still empty.
  const [opened, setOpened] = useState<string[]>([])
  const [needle, setNeedle] = useState('')
  const [stateFilter, setStateFilter] = useState<'all' | 'canon' | 'draft' | 'open'>('all')
  const [folds, setFolds] = useFoldedAll()
  const [summaryFolded, setSummaryFolded] = useState(false)
  const [title, setTitle] = useState(card.title ?? '')
  const [aliases, setAliases] = useState(card.aliases.join(', '))
  const [editingNote, setEditingNote] = useState(false)

  const settle = useCallback(() => {
    for (const key of [...refresh.canon, keys.journal])
      void client.invalidateQueries({ queryKey: key })
  }, [client])
  const body = useNoteBody(card, settle, t('toast.cardSaveFailed'))

  const patch = useAppMutation({
    mutationFn: (change: NotePatch) => updateNote(card.id, change),
    failure: 'toast.cardSaveFailed',
    refresh: refresh.canon,
  })

  const remove = useAppMutation({
    mutationFn: async () => {
      await body.flush()
      return deleteNote(card.id)
    },
    failure: 'toast.cardSaveFailed',
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('canon.cardDeleted', { title: card.title ?? '' }),
        refresh: refresh.canon,
      })
      onGone()
    },
  })

  const task = useAppMutation({
    mutationFn: (action: string) => startCardTask(card.id, action),
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) => assistant.open(started.chatId),
  })
  const gatherAction = cardAction(config.prompts ?? [], 'canon')
  const describeAction = cardAction(config.prompts ?? [], 'card-prompt')
  const summarizeAction = cardAction(config.prompts ?? [], 'card-note')

  const portrait = useMemo(
    () => [...view.pictures].reverse().find((picture) => picture.kind === 'portrait')?.path ?? null,
    [view.pictures],
  )

  if (kind?.root === true) {
    return <ChannelView view={view} lens={lens} onOpen={onOpen} onGone={onGone} />
  }

  const sections = kind?.sections ?? []
  const countOf = (key: string, shape: string | undefined) =>
    shape === 'relations'
      ? view.relations.filter((relation) => relation.section === key).length
      : shape === 'appearances'
        ? view.appearances.length
        : factsIn(view.facts, key).length
  // The description sits under the first section it is written from.
  const describedUnder = kind?.describe_from?.[0]
  // Only the sections that hold something, and those just opened (v0.141,
  // FR-18 18.2); the section a description is written under stays while
  // there is a description. The rest wait under "+ Add section" - all but
  // those turned off in the profile.
  const holds = (one: (typeof sections)[number]) =>
    countOf(one.key, one.shape) > 0 ||
    opened.includes(one.key) ||
    (one.key === describedUnder && (card.prompt ?? '') !== '')
  // A search inside the card and a filter by state, once it holds ten facts
  // (v0.142, FR-18 18.3): the facts that match, in the sections that hold
  // them, opened whatever their fold.
  const searching = needle.trim() !== '' || stateFilter !== 'all'
  const matching = (one: (typeof view.facts)[number]) =>
    (stateFilter === 'all' || one.fact.status === stateFilter) &&
    one.fact.body.toLowerCase().includes(needle.trim().toLowerCase())
  const factsOf = (key: string) =>
    searching ? factsIn(view.facts, key).filter(matching) : undefined
  const shown = sections.filter((one) =>
    searching
      ? (one.shape ?? 'facts') !== 'relations' &&
        one.shape !== 'appearances' &&
        (factsOf(one.key) ?? []).length > 0
      : holds(one),
  )
  // Folds are remembered by the kind of card, so the next card of the kind
  // opens as this one was left.
  const foldId = (key: string) => `canon:${card.kind}:${key}`
  const addable = sections.filter((one) => !holds(one) && one.off !== true)
  const detailed = isDetailed(view, lens)
  // Facts under a section the profile no longer names still read, at the end.
  const orphans = view.facts.filter(
    (read) => !sections.some((one) => one.key === read.fact.section),
  )

  const saveTitle = () => {
    const next = title.trim()
    if (next === '' || next === (card.title ?? '')) {
      setTitle(card.title ?? '')
      return
    }
    patch.mutate({ title: next })
  }
  const saveAliases = () => {
    const next = aliases
      .split(',')
      .map((one) => one.trim())
      .filter((one) => one !== '')
    if (next.join('\u0000') === card.aliases.join('\u0000')) return
    patch.mutate({ aliases: next })
  }

  const actions: RowAction[] = [
    ...(card.work_id !== null
      ? [
          {
            key: 'raise',
            label: t('canon.raise'),
            onSelect: () => patch.mutate({ work_id: null }),
          },
        ]
      : []),
    ...(gatherAction !== undefined
      ? [
          {
            key: 'gather',
            label: sayLabel(gatherAction.label),
            onSelect: () => void body.flush().then(() => task.mutate(gatherAction.key)),
          },
        ]
      : []),
    {
      key: 'delete',
      label: t('canon.deleteCard'),
      danger: true,
      onSelect: () => remove.mutate(),
    },
  ]

  // A hero seen in three works has outgrown one song: the offer to raise it
  // is said out loud then, not only kept in the menu.
  const outgrown = card.work_id !== null && view.appearances.length >= 3

  return (
    <Pane
      label={card.title ?? t('canon.untitled')}
      bodyClassName="flex flex-col px-3.5 pb-4"
      head={
        <div className="flex w-full flex-col gap-2">
          <div className="flex items-center gap-2.5">
            <CardAvatar title={card.title} portrait={portrait} size="lg" />
            <div className="flex min-w-0 flex-1 flex-col">
              <Input
                value={title}
                onChange={(event) => setTitle(event.target.value)}
                onBlur={saveTitle}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') event.currentTarget.blur()
                }}
                aria-label={t('canon.name')}
                className="border-transparent bg-transparent px-1 text-lg font-semibold hover:border-line focus:border-line"
              />
              <div className="flex flex-wrap items-center gap-1.5 px-1 text-xs text-faint">
                <span>{kind === undefined ? card.kind : sayLabel(kind.label)}</span>
                <span aria-hidden>·</span>
                <input
                  value={aliases}
                  onChange={(event) => setAliases(event.target.value)}
                  onBlur={saveAliases}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') event.currentTarget.blur()
                  }}
                  placeholder={t('canon.aliasesPlaceholder')}
                  aria-label={t('canon.aliases')}
                  title={t('canon.aliasesHint')}
                  className="min-w-40 flex-1 rounded-sm bg-transparent px-1 outline-none hover:bg-soft focus:bg-soft"
                />
                {card.work_id !== null && (
                  <Button
                    variant="link"
                    className="text-xs"
                    onClick={() => void navigate(`/works/${card.work_id ?? ''}`)}
                  >
                    {t('canon.heroOf', { title: view.work_title ?? '' })}
                    <ArrowUpRight aria-hidden />
                  </Button>
                )}
              </div>
            </div>
            <SaveState
              savingLabel={t('save.saving')}
              savedLabel={t('save.saved')}
              status={body.status === 'saving' || patch.isPending ? 'saving' : body.status}
            />
            <Select
              value={card.layer}
              onChange={(next) => {
                if (next !== card.layer) patch.mutate({ layer: next as Layer })
              }}
              options={LAYERS.map((layer) => ({
                value: layer,
                label: t(`canon.cardLayer.${layer}`),
              }))}
              aria-label={t('canon.cardLayerLabel')}
              className="w-40"
            />
            <RowMenu actions={actions} label={t('canon.cardActions')} />
          </div>
          {outgrown && (
            <p className="rounded-md bg-accent-soft px-2 py-1 text-xs text-accent-2">
              {t('canon.outgrown', { count: view.appearances.length })}{' '}
              <Button
                variant="link"
                className="text-xs"
                onClick={() => patch.mutate({ work_id: null })}
              >
                {t('canon.raise')}
              </Button>
            </p>
          )}
          <ProposalsBanner view={view} />
          {view.facts.length >= SEARCHABLE && (
            <div className="flex flex-wrap items-center gap-2">
              <Input
                value={needle}
                onChange={(event) => setNeedle(event.target.value)}
                placeholder={t('canon.searchCard')}
                aria-label={t('canon.searchCard')}
                className="min-w-48 flex-1"
              />
              <ChipGroup
                aria-label={t('canon.filter')}
                value={[stateFilter]}
                onValueChange={(next) =>
                  setStateFilter((next[0] ?? 'all') as 'all' | 'canon' | 'draft' | 'open')
                }
              >
                {(['all', 'canon', 'draft', 'open'] as const).map((one) => (
                  <Chip key={one} value={one}>
                    {t(`canon.filterName.${one}`)}
                  </Chip>
                ))}
              </ChipGroup>
            </div>
          )}
          {!searching && shown.length > 1 && (
            <div className="flex gap-1.5">
              <Button
                size="xs"
                variant="ghost"
                onClick={() =>
                  setFolds(
                    shown.map((one) => foldId(one.key)),
                    true,
                  )
                }
              >
                {t('canon.foldAll')}
              </Button>
              <Button
                size="xs"
                variant="ghost"
                onClick={() =>
                  setFolds(
                    shown.map((one) => foldId(one.key)),
                    false,
                  )
                }
              >
                {t('canon.unfoldAll')}
              </Button>
            </div>
          )}
        </div>
      }
    >
      {/* The summary first, under the name (v0.142, FR-18 18.3): what the
          card is, read before its facts - written by hand, or gathered from
          the facts by the assistant; and the facts gathered out of it. */}
      <section className="pt-2" aria-label={t('canon.summary')}>
        <header className="mb-1 flex flex-wrap items-center gap-2">
          <Button
            variant="icon"
            size="icon-xs"
            aria-expanded={!summaryFolded}
            aria-label={t(summaryFolded ? 'canon.unfoldSection' : 'canon.foldSection', {
              section: t('canon.summary'),
            })}
            onClick={() => setSummaryFolded(!summaryFolded)}
          >
            {summaryFolded ? <ChevronRight aria-hidden /> : <ChevronDown aria-hidden />}
          </Button>
          <b className="text-sm font-semibold">{t('canon.summary')}</b>
          <span className="text-xs text-faint">{t('canon.noteHint')}</span>
          <span className="ml-auto flex flex-wrap gap-1.5">
            {summarizeAction !== undefined && (
              <Button
                size="xs"
                variant="ghost"
                disabled={task.isPending}
                onClick={() => void body.flush().then(() => task.mutate(summarizeAction.key))}
              >
                <Sparkles aria-hidden />
                {t('canon.summarize')}
              </Button>
            )}
            {gatherAction !== undefined && body.text.trim() !== '' && (
              <Button
                size="xs"
                variant="ghost"
                disabled={task.isPending}
                onClick={() => void body.flush().then(() => task.mutate(gatherAction.key))}
              >
                <ListPlus aria-hidden />
                {t('canon.extractFacts')}
              </Button>
            )}
            <Button
              size="xs"
              variant="ghost"
              aria-pressed={editingNote}
              onClick={() => {
                if (editingNote) void body.flush()
                setEditingNote(!editingNote)
              }}
            >
              <Pencil aria-hidden />
              {editingNote ? t('notes.done') : t('notes.edit')}
            </Button>
          </span>
        </header>
        {!summaryFolded &&
          (editingNote ? (
            <NoteBodyEditor
              noteId={card.id}
              autoFocus
              autoResize
              rows={4}
              value={body.text}
              onText={(text) => body.setText(text)}
              onKeyDown={(event) => {
                if (event.key === 'Escape') {
                  void body.flush()
                  setEditingNote(false)
                }
              }}
              aria-label={t('canon.summary')}
              className="font-mono text-sm leading-relaxed"
            />
          ) : body.text.trim() === '' ? (
            <p className="px-2 text-xs text-faint">{t('canon.noteEmpty')}</p>
          ) : (
            <div className="selectable px-2" onDoubleClick={() => setEditingNote(true)}>
              <Markdown body={body.text} className="text-sm" />
            </div>
          ))}
      </section>

      <Detailed.Provider value={detailed}>
        {shown.map((one) => (
          <div key={one.key}>
            <SectionBlock
              view={view}
              section={one}
              lens={lens}
              onOpen={onOpen}
              facts={factsOf(one.key)}
              startAdding={opened.includes(one.key) && countOf(one.key, one.shape) === 0}
              folded={!searching && folds.has(foldId(one.key))}
              onFold={searching ? undefined : (fold) => setFolds([foldId(one.key)], fold)}
            />
            {one.key === describedUnder && (
              <PromptBox
                view={view}
                onDescribe={
                  describeAction === undefined ? undefined : () => task.mutate(describeAction.key)
                }
                describing={task.isPending}
              />
            )}
          </div>
        ))}

        {addable.length > 0 && (
          <Menu>
            <MenuTrigger render={<Button size="sm" variant="ghost" className="mt-2 self-start" />}>
              <Plus aria-hidden />
              {t('canon.addSection')}
            </MenuTrigger>
            <MenuPopup align="start">
              {addable.map((one) => (
                <MenuItem key={one.key} onClick={() => setOpened([...opened, one.key])}>
                  {sayLabel(one.label)}
                </MenuItem>
              ))}
            </MenuPopup>
          </Menu>
        )}
        {orphans.length > 0 && (
          <SectionBlock
            view={view}
            section={{ key: '', label: t('canon.otherSection') }}
            facts={orphans}
            lens={lens}
            onOpen={onOpen}
          />
        )}
      </Detailed.Provider>
    </Pane>
  )
}
