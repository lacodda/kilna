import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { FramePrompts, Frame as WorkFrame, FrameView, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { CopyButton } from '@/components/ui/copy-button'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { DetailSwitches } from '@/features/cover/DetailSwitches'
import { FramingPicker } from '@/features/cover/FramingPicker'
import { SchemeView } from '@/features/cover/SchemeView'
import { DraftText, Section } from '@/features/cover/Section'
import { ReleaseMetaStatus } from '@/features/work/ReleaseMetaStatus'
import { useCoverEdit } from '@/features/work/tabs/cover/useCoverEdit'

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
 * Since v0.88 the still is built from the work's cover without its words
 * by default - the same idea, hero, style, ground and mark, no title and no
 * dressing - in a layout of its own when the cover's does not suit a
 * picture with nothing written on it. A still written whole, as every frame
 * was before, stays the person's ("own scene"). The loop's settings are as
 * they were: how long one turn runs, whether the camera holds, whether the
 * last frame meets the first.
 *
 * Laid out as the mockup's s-frame: what is chosen on the left, what is
 * copied on the right. The prompts and the scheme are written on the Rust
 * side (`frame_view`), so what is copied here is what an agent gets too.
 */
export function FrameTab({ work }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const cover = useCoverEdit(work)
  const view = useQuery(queries.frameView(work.id))

  const save = useAppMutation({
    mutationFn: (frame: WorkFrame) => updateWork(work.id, { frame }),
    failure: 'frame.saveFailed',
    refresh: [keys.work(work.id), keys.pictures],
    onSuccess: (updated) => client.setQueryData(keys.work(work.id), updated),
  })

  /**
   * Change a part of the frame and send the whole of it, made over the frame
   * in the cache - two switches pressed in a row both land - and written back
   * before the save goes. A refusal reads the work again.
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
  const built = view.data?.from_cover ?? frame.built ?? frame.still.trim() === ''

  return (
    <Frame>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
        <ReleaseMetaStatus work={work} />
        <div className="flex min-h-0 min-w-0 flex-1 gap-2.5">
          <div className="flex min-h-0 w-98 shrink-0 flex-col">
            <Pane label={t('card.tab.frame')} bodyClassName="flex flex-col px-3 pb-1">
              <Section title={t('frame.source.title')}>
                <SegmentedControl
                  aria-label={t('frame.source.title')}
                  value={built ? 'cover' : 'own'}
                  onValueChange={(next) => change({ built: next === 'cover' })}
                >
                  <Segment value="cover">{t('frame.source.cover')}</Segment>
                  <Segment value="own">{t('frame.source.own')}</Segment>
                </SegmentedControl>
                <span className="text-xs text-faint">
                  {built ? t('frame.source.coverHint') : t('frame.source.ownHint')}
                </span>
              </Section>

              {built ? (
                <>
                  <Section
                    title={t('frame.framing.title')}
                    hint={
                      frame.framing === null ? t('frame.framing.asCover') : t('frame.framing.own')
                    }
                    actions={
                      frame.framing !== null && (
                        <Button
                          variant="link"
                          className="text-xs"
                          onClick={() => change({ framing: null })}
                        >
                          {t('frame.framing.backToCover')}
                        </Button>
                      )
                    }
                  >
                    <FramingPicker
                      framing={frame.framing ?? work.cover.framing}
                      layouts={view.data?.layouts ?? []}
                      lettering={false}
                      onChange={(framing) => change({ framing })}
                    />
                  </Section>
                  <Section title={t('frame.ownWords')} hint={t('frame.ownWordsHint')}>
                    <DraftText
                      label={t('frame.ownWords')}
                      value={frame.still}
                      rows={2}
                      mono
                      onCommit={(still) => change({ still })}
                    />
                  </Section>
                  <Section title={t('cover.details.title')} hint={t('frame.detailsHint')}>
                    <DetailSwitches
                      details={view.data?.details ?? []}
                      onToggle={(id, on) =>
                        cover.change((c) => ({ ...c, details: { ...c.details, [id]: on } }))
                      }
                    />
                  </Section>
                </>
              ) : (
                <Section title={t('frame.still')} hint={t('frame.stillHint')}>
                  <DraftText
                    label={t('frame.still')}
                    value={frame.still}
                    rows={5}
                    mono
                    onCommit={(still) => change({ still })}
                  />
                </Section>
              )}

              <Section title={t('frame.loop')} hint={t('frame.loopHint')}>
                <DraftText
                  label={t('frame.motion')}
                  value={frame.motion}
                  rows={2}
                  mono
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

              <Section
                title={built ? t('frame.negativeOwn') : t('frame.negative')}
                hint={t('frame.negativeHint')}
              >
                <DraftText
                  label={t('frame.negative')}
                  value={frame.negative}
                  rows={2}
                  mono
                  onCommit={(negative) => change({ negative })}
                />
              </Section>
            </Pane>
          </div>

          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            {/* `fill`: the prompts are a pane that scrolls against the
                column's height, and a bare wrapper between them hands it none -
                the pane grew with three long prompts and was cut off at the
                card's edge with no bar (v0.90.1). */}
            <Loaded query={view} skeleton={<SkeletonList rows={3} secondary={false} />} plain fill>
              {(data) => <FramePanel view={data} />}
            </Loaded>
          </div>
        </div>
      </div>
    </Frame>
  )
}

/** The scheme of a built still, then the three prompts to copy. */
function FramePanel({ view }: { view: FrameView }) {
  const { t } = useTranslation()
  return (
    <Pane
      label={t('frame.prompts')}
      head={
        <>
          <b className="text-sm font-semibold">{t('frame.shape', { format: view.format })}</b>
          <span className="text-2xs text-faint">{t('frame.shapeHint')}</span>
        </>
      }
      bodyClassName="flex flex-col gap-3 p-3"
    >
      {view.from_cover && (
        <div className="flex h-[clamp(9rem,28vh,17rem)] items-center justify-center rounded-md bg-soft p-2.5">
          {view.scheme === null ? (
            <p className="max-w-80 text-center text-sm text-faint">{t('frame.noScheme')}</p>
          ) : (
            <SchemeView scheme={view.scheme} label={t('cover.schemeLabel')} fit />
          )}
        </div>
      )}
      {PROMPTS.map((part) => {
        const name = t(`frame.prompt.${part}`)
        const text = view.prompts[part]
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
      })}
    </Pane>
  )
}

/** The lengths offered, with the stored one among them when it is another
 *  number: a loop written as five seconds elsewhere is shown as five, not as
 *  no choice at all. */
function lengthsWith(seconds: number): number[] {
  return LENGTHS.includes(seconds) ? LENGTHS : [...LENGTHS, seconds].sort((a, b) => a - b)
}
