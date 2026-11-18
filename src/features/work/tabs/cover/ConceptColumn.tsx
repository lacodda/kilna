import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import type { ContractFix, Cover, CoverBoard, CoverView } from '@/lib/api/types'
import { fixCover } from '@/lib/coverFix'
import { LAYERS, type Layer } from '@/lib/layers'
import { queries } from '@/lib/query/queries'
import { styleName } from '@/lib/styleBrick'
import { say as sayLabel, styleTypesOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Pane } from '@/components/ui/frame'
import { BrickPick } from '@/features/cover/BrickPick'
import { DetailSwitches } from '@/features/cover/DetailSwitches'
import { FramingPicker } from '@/features/cover/FramingPicker'
import { HeroPick } from '@/features/cover/HeroPick'
import { DraftText, Section } from '@/features/cover/Section'
import { DiscSwitch, GroundPick, SubjectPick } from '@/features/cover/Staging'
import { AccentPick } from '@/features/work/tabs/cover/AccentPick'
import { BackgroundPick } from '@/features/work/tabs/cover/BackgroundPick'
import { CaptionSlots } from '@/features/work/tabs/cover/CaptionSlots'
import { MarkPick } from '@/features/work/tabs/cover/MarkPick'
import { TitleField } from '@/features/work/tabs/cover/TitleField'
import { Shortlist } from '@/features/work/tabs/cover/Shortlist'
import { CoverProblems } from '@/features/work/tabs/cover/CoverNotes'

interface Props {
  cover: Cover
  view: CoverView | undefined
  /** The board of ideas, for the shortlist across the idea's block. */
  board: CoverBoard | undefined
  change: (patch: (cover: Cover) => Cover) => void
  /** Back to the board of ideas. */
  onAllIdeas: () => void
}

/**
 * What a cover is built from, top to bottom in the order a person decides
 * it: the idea and the scene, the hero, the built frame, the style, the
 * words and their dressing, the ground and the accent, the mark, the
 * channel's details. Every choice is saved as it is made - the cover is the
 * record, and the prompt beside it is written from it.
 */
export function ConceptColumn({ cover, view, board, change, onAllIdeas }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const house = view?.house_styles ?? []
  // Names for the folded sections' one-line summaries.
  const bricks = useQuery(queries.styleBricksMatching(null, ''))
  const nameOf = (id: string | null | undefined) => {
    if (id == null) return undefined
    const brick = (bricks.data ?? []).find((one) => one.id === id)
    return brick === undefined ? undefined : styleName(brick)
  }
  const said = (...parts: (string | null | undefined)[]) =>
    parts.filter((part): part is string => part != null && part.trim() !== '').join(' · ')
  // The layers this craft makes, in the order a prompt says them (v0.135).
  const layers = LAYERS.flatMap((key) => {
    const type = styleTypesOf(config).find((one) => one.key === key)
    return type === undefined || (type.retired !== undefined && type.retired !== null) ? [] : [type]
  })
  const problemsOf = (place: string) =>
    (view?.problems ?? []).filter((problem) => 'place' in problem && problem.place === place)
  const fix = (one: ContractFix) => change((c) => fixCover(c, one))

  return (
    <Pane label={t('cover.concept')} bodyClassName="flex flex-col px-3 pb-1">
      {/* Held at the top while the column scrolls (v0.135). */}
      {board !== undefined && <Shortlist board={board} cover={cover} />}
      <Section
        id="cover.idea"
        title={t('cover.idea.title')}
        hint={t('cover.idea.hint')}
        summary={said(cover.idea)}
        actions={
          <Button variant="link" className="text-xs" onClick={onAllIdeas}>
            {t('ideas.all')}
          </Button>
        }
      >
        <DraftText
          label={t('cover.idea.title')}
          value={cover.idea}
          rows={2}
          placeholder={t('cover.idea.placeholder')}
          onCommit={(idea) => change((c) => ({ ...c, idea }))}
        />
        <span className="caption">{t('cover.scene.title')}</span>
        <DraftText
          label={t('cover.scene.title')}
          value={cover.scene}
          rows={3}
          mono
          placeholder={t('cover.scene.placeholder')}
          onCommit={(scene) => change((c) => ({ ...c, scene }))}
        />
        <span className="caption">{t('cover.avoid.title')}</span>
        <DraftText
          label={t('cover.avoid.title')}
          value={cover.avoid}
          rows={2}
          mono
          placeholder={t('cover.avoid.placeholder')}
          onCommit={(avoid) => change((c) => ({ ...c, avoid }))}
        />
      </Section>

      <Section
        id="cover.hero"
        title={t('cover.hero.title')}
        hint={t('cover.hero.hint')}
        summary={said(
          view?.hero?.title ??
            (cover.staging.subject === null ? null : t(`cover.subject.${cover.staging.subject}`)),
        )}
      >
        <HeroPick
          hero={cover.hero}
          state={view?.hero ?? null}
          onChange={(hero) => change((c) => ({ ...c, hero }))}
        />
        {/* With no hero the picture is of what is said here; with one it is
            of a person, and there is nothing to ask. */}
        {cover.hero === null && (
          <SubjectPick
            subject={cover.staging.subject}
            onChange={(subject) => change((c) => ({ ...c, staging: { ...c.staging, subject } }))}
          />
        )}
      </Section>

      <Section
        id="cover.framing"
        title={t('cover.framing.title')}
        hint={t('cover.framing.hint')}
        summary={said(
          cover.framing === null ? null : t(`cover.layout.${cover.framing.layout}`),
          cover.framing === null ? null : t(`cover.crop.${cover.framing.crop}`),
        )}
        actions={
          cover.framing !== null && (
            <Button
              variant="link"
              className="text-xs"
              onClick={() => change((c) => ({ ...c, framing: null }))}
            >
              {t('cover.framing.clear')}
            </Button>
          )
        }
      >
        <FramingPicker
          framing={cover.framing}
          layouts={view?.layouts ?? []}
          lettering
          onChange={(framing) => change((c) => ({ ...c, framing }))}
        />
      </Section>

      <Section
        id="cover.style"
        title={t('cover.style.title')}
        summary={said(nameOf(cover.bricks.style))}
      >
        <BrickPick
          label={t('cover.style.title')}
          form="picture"
          brickId={cover.bricks.style}
          houseStyles={house}
          onChange={(style) => change((c) => ({ ...c, bricks: { ...c.bricks, style } }))}
        />
        <CoverProblems problems={problemsOf('style')} onFix={fix} />
      </Section>

      {/* The layers above the style (v0.135, FR-07 7.3): each a brick of
          the type of its own name, with what its contract says beside it. */}
      {layers.length > 0 && (
        <Section
          id="cover.layers"
          title={t('cover.layers.title')}
          hint={t('cover.layers.hint')}
          summary={said(...layers.map((type) => nameOf(cover.bricks[type.key as Layer])))}
        >
          {layers.map((type) => (
            <div key={type.key} className="flex flex-col gap-1">
              <span className="caption">{sayLabel(type.label)}</span>
              <BrickPick
                label={sayLabel(type.label)}
                form="picture"
                typeKey={type.key}
                brickId={cover.bricks[type.key as Layer] ?? null}
                houseStyles={house}
                onChange={(id) =>
                  change((c) => ({ ...c, bricks: { ...c.bricks, [type.key]: id } }))
                }
              />
              <CoverProblems problems={problemsOf(type.key)} onFix={fix} />
            </div>
          ))}
        </Section>
      )}

      <Section
        id="cover.lettering"
        title={t('cover.lettering.title')}
        hint={t('cover.lettering.hint')}
        summary={said(cover.lettering.title ?? view?.source_title, nameOf(cover.bricks.typography))}
      >
        <TitleField
          title={cover.lettering.title}
          sourceTitle={view?.source_title ?? ''}
          onChange={(title) => change((c) => ({ ...c, lettering: { ...c.lettering, title } }))}
        />
        <BrickPick
          label={t('cover.lettering.title')}
          form="lettering"
          brickId={cover.bricks.typography}
          houseStyles={house}
          onChange={(typography) => change((c) => ({ ...c, bricks: { ...c.bricks, typography } }))}
        />
      </Section>

      <Section
        id="cover.dressing"
        title={t('cover.dressing.title')}
        hint={t('cover.dressing.hint')}
        summary={said(nameOf(cover.bricks.dressing))}
      >
        <BrickPick
          label={t('cover.dressing.title')}
          form="dressing"
          brickId={cover.bricks.dressing}
          houseStyles={house}
          onChange={(dressing) => change((c) => ({ ...c, bricks: { ...c.bricks, dressing } }))}
        />
        <CaptionSlots
          slots={view?.slots ?? []}
          onChange={(slot, lines) =>
            change((c) => {
              const captions = { ...c.lettering.captions }
              if (lines.length === 0) delete captions[slot]
              else captions[slot] = lines
              return { ...c, lettering: { ...c.lettering, captions } }
            })
          }
        />
      </Section>

      <Section
        id="cover.ground"
        title={t('cover.ground.title')}
        summary={said(nameOf(cover.bricks.background), cover.accent?.name)}
      >
        <BackgroundPick
          brickId={cover.bricks.background}
          onChange={(background) => change((c) => ({ ...c, bricks: { ...c.bricks, background } }))}
        />
        {cover.bricks.background !== null && (
          <GroundPick
            ground={cover.staging.ground}
            onChange={(ground) => change((c) => ({ ...c, staging: { ...c.staging, ground } }))}
          />
        )}
        <span className="caption">{t('cover.accent.title')}</span>
        <AccentPick
          accent={cover.accent}
          palette={view?.palette ?? []}
          onChange={(accent) => change((c) => ({ ...c, accent }))}
        />
        {cover.accent !== null && (
          <DiscSwitch
            disc={cover.staging.disc}
            onChange={(disc) => change((c) => ({ ...c, staging: { ...c.staging, disc } }))}
          />
        )}
      </Section>

      <Section
        id="cover.mark"
        title={t('cover.mark.title')}
        hint={t('cover.mark.hint')}
        actions={<CanonLink />}
        summary={said((view?.marks ?? []).find((option) => option.id === cover.mark.variant)?.name)}
      >
        <MarkPick
          mark={cover.mark}
          options={view?.marks ?? []}
          onChange={(mark) => change((c) => ({ ...c, mark }))}
        />
      </Section>

      <Section
        id="cover.details"
        title={t('cover.details.title')}
        hint={t('cover.details.hint', { count: view?.details.length ?? 0 })}
        summary={t('cover.details.on', {
          count: (view?.details ?? []).filter((detail) => detail.on).length,
        })}
        actions={<CanonLink />}
      >
        <DetailSwitches
          details={view?.details ?? []}
          onToggle={(id, on) => change((c) => ({ ...c, details: { ...c.details, [id]: on } }))}
        />
      </Section>
    </Pane>
  )
}

/** Where the channel's card is changed: the Canon screen. */
function CanonLink() {
  const { t } = useTranslation()
  return (
    <Button variant="link" className="text-xs" render={<Link to="/canon" />}>
      {t('cover.toCanon')}
    </Button>
  )
}
