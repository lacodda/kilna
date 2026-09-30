import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { FramePrompts, Frame as WorkFrame, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Checkbox } from '@/components/ui/checkbox'
import { CopyButton } from '@/components/ui/copy-button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { ReleaseMetaStatus } from '@/features/work/ReleaseMetaStatus'
import { coverFormatsOf } from '@/features/work/tabs/cover/cover'

interface Props {
  work: Work
}

/** The lengths a loop is offered in, in seconds: the mockup's four. */
const LENGTHS = [4, 6, 8, 10]

/** The frame's three prompts, in the order they are pasted into a generator. */
const PROMPTS = ['still', 'loop', 'negative'] as const satisfies readonly (keyof FramePrompts)[]

/**
 * The picture a work plays under for its whole length (v0.86, ADR 0046): a
 * still, and a loop made from it.
 *
 * An audio release goes out on a video platform as a track under one picture.
 * What moves in it is one small thing - the light, the water - turning round
 * and round, so the settings of the loop are few and the same every time:
 * how long one turn runs, whether the camera holds, whether the last frame
 * meets the first.
 *
 * Laid out as the mockup's s-frame: what is written on the left, what is
 * copied on the right. The prompts are written on the Rust side from the
 * settings (`frame_prompts`) - the loop's sentence is built there, so what
 * is copied here is what an agent reading the work over MCP would get too.
 *
 * The whole frame travels with every change, the way the cover's blocks do:
 * the log's `before` then holds the frame as it was, and an undo puts all of
 * it back.
 */
export function FrameTab({ work }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()

  const save = useAppMutation({
    mutationFn: (frame: WorkFrame) => updateWork(work.id, { frame }),
    failure: 'frame.saveFailed',
    refresh: [keys.work(work.id), keys.framePromptsFor(work.id)],
    // The work as the backend now holds it, at once.
    onSuccess: (updated) => client.setQueryData(keys.work(work.id), updated),
  })

  /**
   * Change a part of the frame and send the whole of it.
   *
   * Made over the frame in the cache, not the one this render was handed,
   * and written back into the cache before the save goes: two switches
   * pressed in a row then both land, rather than the second carrying the
   * first one's old value back - a render between two clicks is not
   * promised. The switches show the change at once for the same reason. A
   * refusal reads the work again, so what is shown is what is stored.
   */
  const change = (patch: Partial<WorkFrame>) => {
    const held = client.getQueryData<Work | null>(keys.work(work.id))
    const next = { ...(held?.frame ?? work.frame), ...patch }
    if (held != null) client.setQueryData<Work>(keys.work(work.id), { ...held, frame: next })
    save.mutate(next, {
      onError: () => void client.invalidateQueries({ queryKey: keys.work(work.id) }),
    })
  }
  const { frame } = work

  // The shape of the picture is the door's, not the frame's: the first door
  // of the kind that says what shape its picture is.
  const format = coverFormatsOf(profile.config, work.kind)[0]?.format

  return (
    <Frame>
      {/* One column with its own gap, as on the Cover tab: the status draws
          nothing when nothing is being written. */}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
        <ReleaseMetaStatus work={work} />
        <div className="flex min-h-0 min-w-0 flex-1 gap-2.5">
          {/* The mockup's left column is a fixed width, as a list's is: the
              settings read the same on a wide window and the prompts take
              what is left. */}
          <div className="flex min-h-0 w-98 shrink-0 flex-col">
            <Pane label={t('card.tab.frame')} bodyClassName="flex flex-col px-3 pb-1">
              <Section title={t('frame.still')} hint={t('frame.stillHint')}>
                <FrameText
                  label={t('frame.still')}
                  value={frame.still}
                  rows={5}
                  onCommit={(still) => change({ still })}
                />
              </Section>

              <Section title={t('frame.loop')} hint={t('frame.loopHint')}>
                <FrameText
                  label={t('frame.motion')}
                  value={frame.motion}
                  rows={2}
                  placeholder={t('frame.motionPlaceholder')}
                  onCommit={(motion) => change({ motion })}
                />
                <div className="flex flex-wrap items-center gap-2.5">
                  <span className="caption">{t('frame.length')}</span>
                  <SegmentedControl
                    aria-label={t('frame.length')}
                    value={String(frame.seconds)}
                    onValueChange={(next) => change({ seconds: Number(next) })}
                  >
                    {lengthsWith(frame.seconds).map((seconds) => (
                      <Segment key={seconds} value={String(seconds)}>
                        {t('frame.seconds', { seconds })}
                      </Segment>
                    ))}
                  </SegmentedControl>
                </div>
                <Checkbox
                  checked={frame.still_camera}
                  onCheckedChange={(still_camera) => change({ still_camera })}
                >
                  {t('frame.stillCamera')}
                </Checkbox>
                <Checkbox
                  checked={frame.seamless}
                  onCheckedChange={(seamless) => change({ seamless })}
                >
                  {t('frame.seamless')}
                </Checkbox>
              </Section>

              <Section title={t('frame.negative')} hint={t('frame.negativeHint')}>
                <FrameText
                  label={t('frame.negative')}
                  value={frame.negative}
                  rows={2}
                  onCommit={(negative) => change({ negative })}
                />
              </Section>
            </Pane>
          </div>

          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            <Pane
              label={t('frame.prompts')}
              head={
                <>
                  <b className="text-sm font-semibold">
                    {format === undefined ? t('frame.shapeless') : t('frame.shape', { format })}
                  </b>
                  <span className="text-2xs text-faint">{t('frame.shapeHint')}</span>
                </>
              }
              bodyClassName="flex flex-col gap-3 p-3"
            >
              <PromptBlocks workId={work.id} />
            </Pane>
          </div>
        </div>
      </div>
    </Frame>
  )
}

/** The lengths offered, with the stored one among them when it is another
 *  number: a loop written as five seconds elsewhere is shown as five, not as
 *  no choice at all. */
function lengthsWith(seconds: number): number[] {
  return LENGTHS.includes(seconds) ? LENGTHS : [...LENGTHS, seconds].sort((a, b) => a - b)
}

/** A block of the settings, the mockup's `csec`: a title and a hint across
 *  the top, a line under it. */
function Section({ title, hint, children }: { title: string; hint: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2.5 border-b border-line py-3 last:border-b-0">
      <header className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
        <b className="text-sm font-semibold">{title}</b>
        <span className="text-2xs text-faint">{hint}</span>
      </header>
      {children}
    </section>
  )
}

/**
 * A text of the frame, held while it is typed and saved on blur - and only
 * when it changed, so tabbing through the boxes writes nothing. A save per
 * keystroke would write half-words and redraw the box under the cursor (the
 * cover's blocks say the same).
 */
function FrameText({
  label,
  value,
  rows,
  placeholder,
  onCommit,
}: {
  label: string
  value: string
  rows: number
  placeholder?: string
  onCommit: (text: string) => void
}) {
  const [text, setText] = useState<string | null>(null)
  return (
    <Textarea
      value={text ?? value}
      rows={rows}
      aria-label={label}
      placeholder={placeholder}
      className="font-mono text-xs leading-relaxed"
      onChange={(event) => setText(event.target.value)}
      onBlur={() => {
        if (text !== null && text !== value) onCommit(text)
        setText(null)
      }}
    />
  )
}

/**
 * The three prompts, each copied on its own: each goes into a different box
 * of whatever draws the picture and animates it.
 */
function PromptBlocks({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const prompts = useQuery(queries.framePrompts(workId))

  return (
    <Loaded query={prompts} skeleton={<SkeletonList rows={3} secondary={false} />} plain>
      {(data) =>
        PROMPTS.map((part) => {
          const name = t(`frame.prompt.${part}`)
          const text = data[part]
          const empty = text.trim() === ''
          return (
            // `group`: the copy button shows while the pointer is over the
            // block it copies, and whenever the keyboard is on it.
            <section key={part} aria-label={name} className="group flex min-w-0 flex-col gap-1.5">
              <header className="flex min-h-5 items-center gap-2">
                <span className="caption">{name}</span>
                <span className="font-mono text-2xs text-faint">{part}</span>
                <CopyButton
                  value={text}
                  label={t('frame.copy', { part: name })}
                  copiedLabel={t('frame.copied')}
                  title={t('frame.copy', { part: name })}
                  disabled={empty}
                  className="ml-auto"
                  onCopy={(ok) => {
                    if (!ok) say.failed(t('work.copyFailed'))
                  }}
                />
              </header>
              {empty ? (
                <p className="text-xs text-faint">{t('frame.nothing')}</p>
              ) : (
                <pre className="selectable rounded-md border border-line bg-soft px-2.5 py-2 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap text-dim">
                  {text}
                </pre>
              )}
            </section>
          )
        })
      }
    </Loaded>
  )
}
