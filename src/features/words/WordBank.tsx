import { useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { TextSearch } from 'lucide-react'
import { addToBlock, bankWords, removeFromBlock, updateTerm } from '@/lib/api/register'
import type { Bank } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useChipDrag, type Dragging } from '@/lib/useChipDrag'
import { ALL_WORDS, blockOf, NO_BLOCK, splitWords, wordsIn, type BankScope } from '@/lib/words'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { SkeletonList } from '@/components/ui/skeleton'
import { ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { BankBlocks } from '@/features/words/BankBlocks'
import { BankWord } from '@/features/words/BankWord'
import { FromTextsDialog } from '@/features/words/FromTextsDialog'

/**
 * The bank of words (ADR 0052): words kept for songs to come, sorted into
 * blocks the owner names and orders - a word in as many blocks as it fits.
 *
 * Not a note kind and not a table of its own: a word of the bank is a word of
 * the record with its bank facet set, so "пульсар" kept here, sung "пульсАр"
 * and spent in the register is one row, and the register's screen reads the
 * same list by its other facets. The Notes screen is where it lives because
 * that is where the owner keeps material for what is not written yet - the
 * lines, the ideas - and this is more of it.
 *
 * Words come in through the line at the top: one and Enter, or a list by
 * commas, or pasted by lines - into the block on show, or into no block.
 * A word goes into a block by being carried onto it; that is a pointer drag
 * (`useChipDrag`), since a desktop shell that takes file drops never lets an
 * HTML5 drag reach the page.
 */
export function WordBank() {
  const { t } = useTranslation()
  const terms = useQuery(queries.terms())
  const blocks = useQuery(queries.blocks())
  const [scope, setScope] = useState<BankScope>(ALL_WORDS)
  const [draft, setDraft] = useState('')
  const [finding, setFinding] = useState(false)
  // The block a carried word is over, lit while it is.
  const [over, setOver] = useState<string | null>(null)

  const entries = useMemo(() => terms.data ?? [], [terms.data])
  const list = useMemo(() => blocks.data ?? [], [blocks.data])
  // A block that went - deleted here, or elsewhere - leaves the whole bank on
  // show rather than a list of nothing under a name that is not there.
  const shown =
    scope === ALL_WORDS || scope === NO_BLOCK || blockOf(scope, list) !== null ? scope : ALL_WORDS
  const block = blockOf(shown, list)
  const rows = useMemo(() => wordsIn(shown, entries, list), [shown, entries, list])
  const counts = useMemo(
    () => ({
      all: wordsIn(ALL_WORDS, entries, list).length,
      loose: wordsIn(NO_BLOCK, entries, list).length,
    }),
    [entries, list],
  )
  const wordOf = (id: string) => entries.find((entry) => entry.id === id)?.word ?? ''

  const bank = useAppMutation({
    mutationFn: (words: string[]) => bankWords(words, block?.id ?? null),
    failure: 'words.bankFailed',
    refresh: refresh.term,
    onSuccess: (banked) => {
      setDraft('')
      // A word already in the record joins the bank rather than being made a
      // second time (ADR 0052), and nothing on the screen would say so: a
      // word that was set aside comes back fresh where it stood.
      const known = banked.terms.length - banked.created
      if (known > 0) say.info(t('words.known', { count: known }))
    },
  })
  const put = useAppMutation({
    mutationFn: ({ blockId, termId }: { blockId: string; termId: string }) =>
      addToBlock(blockId, termId),
    failure: 'words.blockFailed',
    refresh: refresh.term,
    // Said, because the word stays where it was on the right: the list on show
    // is still the one it was carried from.
    onSuccess: (_, { blockId, termId }) =>
      say.ok(t('words.put', { word: wordOf(termId), block: blockOf(blockId, list)?.name ?? '' })),
  })
  const takeOut = useAppMutation({
    mutationFn: ({ blockId, termId }: { blockId: string; termId: string }) =>
      removeFromBlock(blockId, termId),
    failure: 'words.blockFailed',
    refresh: refresh.term,
  })
  const patch = useAppMutation({
    mutationFn: ({ id, next }: { id: string; next: Bank | null }) => updateTerm(id, { bank: next }),
    failure: 'words.saveFailed',
    refresh: refresh.term,
  })

  // The bank has no month to turn: the hook is handed no grid, so the pointer
  // is never near an edge and its timer never starts.
  const noGrid = useRef<HTMLElement>(null)
  const carry = useChipDrag({
    gridRef: noGrid,
    onEdge: () => {},
    onDrop: (termId, target) => {
      setOver(null)
      const blockId = target?.closest<HTMLElement>('[data-block-drop]')?.dataset.blockDrop
      if (blockId === undefined) return
      // Let go over a block that has it already: put back, nothing written.
      if (blockOf(blockId, list)?.term_ids.includes(termId) === true) return
      put.mutate({ blockId, termId })
    },
  })

  const add = () => {
    const words = splitWords(draft)
    if (words.length > 0 && !bank.isPending) bank.mutate(words)
  }
  const filled = rows.length > 0

  return (
    <>
      <ListDetail
        list={
          <BankBlocks
            blocks={blocks}
            scope={shown}
            onScope={setScope}
            counts={counts}
            carrying={carry.dragging !== null}
            over={over}
            onOver={setOver}
          />
        }
        detail={
          <Pane
            label={block?.name ?? t('words.bank')}
            bodyClassName="p-1.5"
            head={
              <>
                <Input
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') add()
                  }}
                  onPaste={(event) => {
                    // A field of one line glues a list pasted by lines into
                    // one word. Its lines become commas instead, where they can
                    // be read - and corrected - before Enter keeps them.
                    const pasted = event.clipboardData.getData('text')
                    if (!/[\n\r]/.test(pasted)) return
                    event.preventDefault()
                    const field = event.currentTarget
                    const from = field.selectionStart ?? draft.length
                    const to = field.selectionEnd ?? draft.length
                    setDraft(draft.slice(0, from) + splitWords(pasted).join(', ') + draft.slice(to))
                  }}
                  placeholder={
                    block === null ? t('words.add') : t('words.addTo', { block: block.name })
                  }
                  aria-label={
                    block === null ? t('words.add') : t('words.addTo', { block: block.name })
                  }
                  className="min-w-48 flex-1"
                />
                <Button size="sm" onClick={() => setFinding(true)}>
                  <TextSearch aria-hidden />
                  {t('words.fromTexts')}
                </Button>
              </>
            }
          >
            <Loaded
              query={terms}
              skeleton={<SkeletonList rows={8} />}
              isEmpty={() => !filled}
              emptyState={
                <EmptyState
                  plain
                  title={
                    block !== null
                      ? t('words.blockEmpty')
                      : shown === NO_BLOCK && counts.all > 0
                        ? t('words.looseEmpty')
                        : t('words.empty')
                  }
                  body={
                    block !== null
                      ? t('words.blockEmptyBody')
                      : shown === NO_BLOCK && counts.all > 0
                        ? undefined
                        : t('words.emptyBody')
                  }
                  className="p-2"
                />
              }
              plain
            >
              {() => (
                <ul className="flex flex-col">
                  {rows.map((entry) => (
                    <BankWord
                      key={entry.id}
                      entry={entry}
                      block={block}
                      blocks={list}
                      carrying={carry.dragging?.id === entry.id}
                      onGrab={(event) => {
                        // A carry let go with Escape left no drop to clear the
                        // block it was over; this one starts over nothing.
                        setOver(null)
                        carry.begin(event, entry.id)
                      }}
                      onBank={(next) => patch.mutate({ id: entry.id, next })}
                      onPut={(blockId) => put.mutate({ blockId, termId: entry.id })}
                      onTakeOut={() => {
                        if (block !== null) takeOut.mutate({ blockId: block.id, termId: entry.id })
                      }}
                    />
                  ))}
                </ul>
              )}
            </Loaded>
          </Pane>
        }
      />
      {carry.dragging !== null && (
        <CarriedWord dragging={carry.dragging} word={wordOf(carry.dragging.id)} />
      )}
      {finding && <FromTextsDialog open onOpenChange={setFinding} />}
    </>
  )
}

/**
 * The word that follows the pointer to a block: the word itself on the
 * panel's ground, fixed to the viewport so no pane's overflow clips it on the
 * way from the right to the left, and deaf to the pointer, so what it is let
 * go over is the block under it rather than itself.
 */
function CarriedWord({ dragging, word }: { dragging: Dragging; word: string }) {
  return createPortal(
    <div
      aria-hidden
      className="pointer-events-none fixed rounded-md border border-line bg-raise px-2 py-1 text-sm font-medium shadow-float [z-index:var(--z-overlay)]"
      style={{ left: dragging.ghost.left, top: dragging.ghost.top }}
    >
      {word}
    </div>,
    document.body,
  )
}
