import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import type { Cover, CoverView } from '@/lib/api/types'
import { Button } from '@/components/ui/button'
import { Pane } from '@/components/frame'
import { BrickPick } from '@/features/cover/BrickPick'
import { DetailSwitches } from '@/features/cover/DetailSwitches'
import { FramingPicker } from '@/features/cover/FramingPicker'
import { HeroPick } from '@/features/cover/HeroPick'
import { DraftText, Section } from '@/features/cover/Section'
import { AccentPick } from '@/features/work/tabs/cover/AccentPick'
import { BackgroundPick } from '@/features/work/tabs/cover/BackgroundPick'
import { CaptionSlots } from '@/features/work/tabs/cover/CaptionSlots'
import { MarkPick } from '@/features/work/tabs/cover/MarkPick'
import { TitleField } from '@/features/work/tabs/cover/TitleField'

interface Props {
  cover: Cover
  view: CoverView | undefined
  change: (patch: (cover: Cover) => Cover) => void
}

/**
 * What a cover is built from, top to bottom in the order a person decides
 * it: the idea and the scene, the hero, the built frame, the style, the
 * words and their dressing, the ground and the accent, the mark, the
 * channel's details. Every choice is saved as it is made - the cover is the
 * record, and the prompt beside it is written from it.
 */
export function ConceptColumn({ cover, view, change }: Props) {
  const { t } = useTranslation()
  const house = view?.house_styles ?? []

  return (
    <Pane label={t('cover.concept')} bodyClassName="flex flex-col px-3 pb-1">
      <Section title={t('cover.idea.title')} hint={t('cover.idea.hint')}>
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
      </Section>

      <Section title={t('cover.hero.title')} hint={t('cover.hero.hint')}>
        <HeroPick
          hero={cover.hero}
          state={view?.hero ?? null}
          onChange={(hero) => change((c) => ({ ...c, hero }))}
        />
      </Section>

      <Section
        title={t('cover.framing.title')}
        hint={t('cover.framing.hint')}
        actions={
          cover.framing !== null && (
            <Button variant="link" onClick={() => change((c) => ({ ...c, framing: null }))}>
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

      <Section title={t('cover.style.title')}>
        <BrickPick
          label={t('cover.style.title')}
          form="picture"
          brickId={cover.bricks.style}
          houseStyles={house}
          onChange={(style) => change((c) => ({ ...c, bricks: { ...c.bricks, style } }))}
        />
      </Section>

      <Section title={t('cover.lettering.title')} hint={t('cover.lettering.hint')}>
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

      <Section title={t('cover.dressing.title')} hint={t('cover.dressing.hint')}>
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

      <Section title={t('cover.ground.title')}>
        <BackgroundPick
          brickId={cover.bricks.background}
          onChange={(background) => change((c) => ({ ...c, bricks: { ...c.bricks, background } }))}
        />
        <span className="caption">{t('cover.accent.title')}</span>
        <AccentPick
          accent={cover.accent}
          palette={view?.palette ?? []}
          onChange={(accent) => change((c) => ({ ...c, accent }))}
        />
      </Section>

      <Section title={t('cover.mark.title')} hint={t('cover.mark.hint')} actions={<CanonLink />}>
        <MarkPick
          mark={cover.mark}
          options={view?.marks ?? []}
          onChange={(mark) => change((c) => ({ ...c, mark }))}
        />
      </Section>

      <Section
        title={t('cover.details.title')}
        hint={t('cover.details.hint', { count: view?.details.length ?? 0 })}
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
    <Button variant="link" render={<Link to="/canon" />}>
      {t('cover.toCanon')}
    </Button>
  )
}
