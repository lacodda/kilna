import { useTranslation } from 'react-i18next'
import type { CoverGround, CoverSubject } from '@/lib/api/types'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Switch } from '@/components/ui/switch'

/*
 * How a picture is staged (v0.115): what it is of, and what its accent and
 * its background are to it. Three small controls, each beside the part of
 * the constructor it speaks about.
 */

/** In the order they are offered: who, what, nobody. */
const SUBJECTS: readonly CoverSubject[] = ['person', 'object', 'none']

/** A word per subject, written out so a new one is a type error here. */
const SUBJECT_KEYS: Record<CoverSubject, string> = {
  person: 'cover.subject.person',
  object: 'cover.subject.object',
  none: 'cover.subject.none',
}

/**
 * What the picture is of when no hero is chosen: a person the scene
 * describes, a thing, or nobody.
 *
 * It decides the words of the frame. Until v0.115 every frame said "the
 * hero" - its head behind the title, its chest under it, its eyes in a
 * close-up - and a cover of an empty street was told where the hero's face
 * goes. With a hero chosen the picture is of a person, and this is not
 * asked.
 */
export function SubjectPick({
  subject,
  disabled,
  onChange,
}: {
  /** As the cover holds it; absent reads as a thing. */
  subject: CoverSubject | null
  disabled?: boolean
  onChange: (subject: CoverSubject) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      <span className="caption">{t('cover.subject.title')}</span>
      <SegmentedControl
        aria-label={t('cover.subject.title')}
        value={subject ?? 'object'}
        disabled={disabled}
        onValueChange={(next) => onChange(next as CoverSubject)}
      >
        {SUBJECTS.map((one) => (
          <Segment key={one} value={one}>
            {t(SUBJECT_KEYS[one])}
          </Segment>
        ))}
      </SegmentedControl>
      <p className="text-2xs text-faint">{t('cover.subject.hint')}</p>
    </div>
  )
}

/**
 * The accent as a flat disc behind the subject, rather than a colour for
 * highlights and edges. Off unless switched on: until v0.115 five layouts of
 * the eight put it behind every picture that had an accent.
 */
export function DiscSwitch({
  disc,
  disabled,
  onChange,
}: {
  disc: boolean
  disabled?: boolean
  onChange: (disc: boolean) => void
}) {
  const { t } = useTranslation()
  return (
    <Switch checked={disc} disabled={disabled} onCheckedChange={onChange}>
      {t('cover.accent.disc')}
    </Switch>
  )
}

const GROUNDS: readonly CoverGround[] = ['palette', 'flat']

const GROUND_KEYS: Record<CoverGround, string> = {
  palette: 'cover.ground.palette',
  flat: 'cover.ground.flat',
}

/**
 * What the chosen background is to the picture: its colours and mood, with
 * the scene keeping the place it describes - or the whole ground, flat, a
 * studio backdrop with no scenery.
 */
export function GroundPick({
  ground,
  disabled,
  onChange,
}: {
  ground: CoverGround
  disabled?: boolean
  onChange: (ground: CoverGround) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      <SegmentedControl
        aria-label={t('cover.ground.role')}
        value={ground}
        disabled={disabled}
        onValueChange={(next) => onChange(next as CoverGround)}
      >
        {GROUNDS.map((one) => (
          <Segment key={one} value={one}>
            {t(GROUND_KEYS[one])}
          </Segment>
        ))}
      </SegmentedControl>
      <p className="text-2xs text-faint">
        {t(ground === 'flat' ? 'cover.ground.flatHint' : 'cover.ground.paletteHint')}
      </p>
    </div>
  )
}
