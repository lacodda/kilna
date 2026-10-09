import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { ArrowLeft, ArrowRight, X } from 'lucide-react'
import type { Composition, StyleBrick } from '@/lib/api/types'
import { brickMatches, offeredBricks } from '@/lib/phrases'
import { queries } from '@/lib/query/queries'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Dialog } from '@/components/AppDialog'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  composition: Composition
  /** The work whose fields close the text. */
  workId: string
  /** Keep the text: it becomes a new version, written by hand from here on -
   *  or a trial, which remembers the bricks it was picked from. */
  onUse?: (text: string, bricks: string[]) => void
  /** What the button that keeps the text says, when not "Use". */
  useLabel?: string
  /** Hand the picks to the assistant, which writes them up as prose. */
  prose?: { label: string; onStart: (bricks: string[]) => void }
}

/**
 * A text written out of the dictionary (v0.94): the composition's blocks in
 * the order a text is built - genre, voice, groove, instruments... - each
 * with its rule and the bricks it offers, the channel's house ones first;
 * and below, the line the picks make.
 *
 * **The order picked is the order written.** A generator weighs its words from
 * the left, so the line is the picks as they were picked, and can be moved
 * - not re-sorted into the blocks' order. The text itself is the backend's:
 * one writer for the window and an agent (`compose_text`), so the same picks
 * are the same text everywhere, the work's fields closing it.
 */
export function ComposeDialog({
  open,
  onOpenChange,
  composition,
  workId,
  onUse,
  useLabel,
  prose,
}: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const types = styleTypesOf(config)
  const [picked, setPicked] = useState<string[]>([])
  const [text, setText] = useState('')
  const query = useDebounced(text, 150)

  const bricks = useQuery(queries.styleBricksMatching(null, ''))
  const house = useQuery(queries.houseStyles())
  const houseSet = useMemo(() => new Set(house.data ?? []), [house.data])
  const byId = useMemo(
    () => new Map((bricks.data ?? []).map((brick) => [brick.id, brick])),
    [bricks.data],
  )
  const written = useQuery({
    ...queries.composedText(composition.key, picked, workId),
    enabled: picked.length > 0,
    placeholderData: keepPreviousData,
  })

  const toggle = (id: string) =>
    setPicked((now) => (now.includes(id) ? now.filter((one) => one !== id) : [...now, id]))
  const move = (from: number, to: number) => {
    if (to < 0 || to >= picked.length) return
    const next = [...picked]
    const [taken] = next.splice(from, 1)
    if (taken === undefined) return
    next.splice(to, 0, taken)
    setPicked(next)
  }

  const result = picked.length === 0 ? undefined : written.data
  const over = result?.limit !== undefined && result.limit !== null && result.length > result.limit
  const problemOf = (typeKey: string) => result?.problems.find((p) => p.type_key === typeKey)

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('compose.title', { name: sayLabel(composition.label) })}
      description={t('compose.body')}
      size="xl"
      dirty={picked.length > 0}
      footer={
        <>
          {prose !== undefined && (
            <Button
              variant={onUse === undefined ? 'primary' : 'soft'}
              disabled={picked.length === 0}
              onClick={() => prose.onStart(picked)}
            >
              {prose.label}
            </Button>
          )}
          {onUse !== undefined && (
            <Button
              variant="primary"
              disabled={result === undefined || result.text === ''}
              onClick={() => {
                if (result !== undefined) onUse(result.text, picked)
              }}
            >
              {useLabel ?? t('compose.use')}
            </Button>
          )}
        </>
      }
    >
      <div className="flex flex-col gap-3">
        <Input
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={t('compose.search')}
          aria-label={t('compose.search')}
        />

        <div className="flex max-h-[46vh] flex-col gap-3 overflow-y-auto pr-1">
          {composition.parts.map((part) => {
            const type = types.find((one) => one.key === part.type)
            const Icon = styleIconOf(type)
            const offered = offeredBricks(bricks.data ?? [], part.type, houseSet).filter(
              (brick) => brickMatches(brick, query) || picked.includes(brick.id),
            )
            const count = picked.filter((id) => byId.get(id)?.type_key === part.type).length
            const problem = problemOf(part.type)
            return (
              <section
                key={part.type}
                aria-label={type === undefined ? part.type : sayLabel(type.label)}
              >
                <header className="mb-1.5 flex flex-wrap items-baseline gap-x-2">
                  <span className="inline-flex items-center gap-1.5 text-sm font-semibold">
                    <Icon aria-hidden className="size-3.5 text-dim" />
                    {type === undefined ? part.type : sayLabel(type.label)}
                  </span>
                  <span
                    className={cn(
                      'font-mono text-xs tabular-nums',
                      problem === undefined ? 'text-faint' : 'text-warn',
                    )}
                  >
                    {part.max === undefined || part.max === null
                      ? t('compose.countOpen', { count, min: part.min })
                      : t('compose.count', { count, min: part.min, max: part.max })}
                  </span>
                  {part.rule !== undefined && part.rule !== null && (
                    <span className="text-xs text-dim">{sayLabel(part.rule)}</span>
                  )}
                </header>
                {offered.length === 0 ? (
                  <p className="text-xs text-faint">{t('compose.noneHere')}</p>
                ) : (
                  <div className="flex flex-wrap gap-1.5">
                    {offered.map((brick) => (
                      <Chip
                        key={brick.id}
                        pressed={picked.includes(brick.id)}
                        onPressedChange={() => toggle(brick.id)}
                        variant={houseSet.has(brick.id) ? 'soft' : 'outline'}
                        title={explainedTitle(
                          brick,
                          houseSet.has(brick.id) ? t('styles.house') : null,
                        )}
                        className="font-mono"
                      >
                        {brick.description ?? brick.name}
                      </Chip>
                    ))}
                  </div>
                )}
              </section>
            )
          })}
        </div>

        {/* The line the picks make, in the order they were picked. */}
        <section
          aria-label={t('compose.line')}
          className="flex flex-col gap-2 border-t border-line pt-3"
        >
          {picked.length === 0 ? (
            <p className="text-sm text-faint">{t('compose.empty')}</p>
          ) : (
            <ol className="flex flex-wrap gap-1.5">
              {picked.map((id, index) => {
                const brick = byId.get(id)
                if (brick === undefined) return null
                return (
                  <li
                    key={id}
                    className="inline-flex items-center gap-0.5 rounded-full border border-line bg-raise py-0.5 pr-0.5 pl-2 text-xs"
                  >
                    <span className="font-mono">{brick.description ?? brick.name}</span>
                    <Button
                      size="icon-xs"
                      variant="icon"
                      aria-label={t('compose.earlier')}
                      disabled={index === 0}
                      onClick={() => move(index, index - 1)}
                    >
                      <ArrowLeft aria-hidden />
                    </Button>
                    <Button
                      size="icon-xs"
                      variant="icon"
                      aria-label={t('compose.later')}
                      disabled={index === picked.length - 1}
                      onClick={() => move(index, index + 1)}
                    >
                      <ArrowRight aria-hidden />
                    </Button>
                    <Button
                      size="icon-xs"
                      variant="icon"
                      aria-label={t('styles.unpick')}
                      onClick={() => toggle(id)}
                    >
                      <X aria-hidden />
                    </Button>
                  </li>
                )
              })}
            </ol>
          )}
          {result !== undefined && (
            <>
              <p className="rounded-md bg-soft px-3 py-2 font-mono text-sm break-words">
                {result.text}
              </p>
              <p className={cn('text-xs tabular-nums', over ? 'text-bad' : 'text-faint')}>
                {result.limit === undefined || result.limit === null
                  ? t('compose.length', { count: result.length })
                  : t('compose.lengthOf', { count: result.length, limit: result.limit })}
              </p>
            </>
          )}
        </section>
      </div>
    </Dialog>
  )
}

/** What a chip says on hover: what the phrase means, and whether it is one of
 *  the channel's own. */
function explainedTitle(brick: StyleBrick, house: string | null): string {
  const meaning = sayLabel(brick.explanation)
  const parts = [meaning, brick.when_to_use ?? '', house ?? ''].filter((part) => part !== '')
  return parts.join(' · ')
}
