import { useTranslation } from 'react-i18next'
import { ChevronRight } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import type { Scene, SceneFrame, SceneNote, ScenePatch } from '@/lib/api/types'
import {
  chosenFrame,
  headlineOf,
  linesAfterHeadline,
  readinessOf,
  withHeadline,
} from '@/lib/scenes'
import { say } from '@/lib/toast'
import { say as sayLabel, type Vocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { InlineField, numberCodec, timecodeCodec } from '@/components/ui/inline-field'
import { Spinner } from '@/components/ui/spinner'
import { TableCell, TableRow } from '@/components/ui/table'
import { Select } from '@/components/AppSelect'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { ReadinessMark } from '@/features/work/tabs/scenes/ReadinessMark'

/** The mockup's cell padding, tighter than a table's default: nine columns
    share a card, and the description is the one that pays for every pixel. */
export const CELL = 'px-2'

interface Props {
  scene: Scene
  vocabulary: Vocabulary
  /** Whether the board has an About column: the heading and the cells are
      drawn on the same condition, or the cells slide under the next heading. */
  aboutShown: boolean
  /** Who is in this scene, where it happens. */
  about: SceneNote[]
  /** The pictures and clips drawn for it, in order. */
  frames: SceneFrame[]
  open: boolean
  onToggle: () => void
  saving: boolean
  onPatch: (changes: ScenePatch) => void
  onViewFrame: (frame: SceneFrame) => void
  /** Put this scene at that number, the rest closing up behind it. */
  onMoveTo: (position: number) => void
  /** How many scenes the board has, so a number past the end is refused
   * here rather than silently understood as the end. */
  total: number
  /** The board is being renumbered: the fields wait rather than queue up a
   * second order against a board that is still moving. */
  moving: boolean
  /** What the row's menu offers: adding beside it, copying, deleting. */
  actions: RowAction[]
}

/**
 * One scene as a row of the board: its number, the part it plays against,
 * its seconds, the kind of shot, the first line of its description, who it is
 * about, and how far it is filled in. Every cell is the value itself, edited
 * where it stands - the mockup's `.cell`: no box until the pointer or the
 * focus is on it, then a hairline and a soft ground - and saved when left;
 * nothing here asks to be saved.
 *
 * What is done TO the row rather than typed into it - a scene beside it, a
 * copy of it, deleting it - is one menu at its end. There were up to eight
 * buttons there, the assistant's among them, and the column they needed was
 * the reason the board could not fit a laptop.
 */
export function SceneRow({
  scene,
  vocabulary,
  aboutShown,
  about,
  frames,
  open,
  onToggle,
  saving,
  onPatch,
  onViewFrame,
  onMoveTo,
  total,
  moving,
  actions,
}: Props) {
  const { t } = useTranslation()

  // Every cell is a draft over what is stored (InlineField): it follows the
  // scene while nobody types in it, writes only a change, and goes back to
  // what is stored when a value is refused - a cell showing an order or a
  // time that was not applied would be a second truth about the scene. What
  // cannot be read as a number or a time at all is refused by the field
  // itself; a number that names no place on the board is refused here, with
  // the range said out loud.
  const moveTo = (wanted: number | null) => {
    if (wanted === null || !Number.isInteger(wanted) || wanted < 1 || wanted > total) {
      say.warn(t('scenes.badNumber', { text: wanted === null ? '' : String(wanted), total }))
      return false
    }
    if (wanted === scene.position) return false
    onMoveTo(wanted)
  }

  const timecode =
    (field: 'starts_at' | 'ends_at', stored: number | null) => (seconds: number | null) => {
      // The same moment typed another way ("0:5") is not a change, and the
      // box goes back to the stored spelling of it.
      if (seconds === stored) return false
      onPatch({ [field]: seconds })
    }

  const section = (text: string | null) => {
    const trimmed = (text ?? '').trim()
    if (trimmed === (scene.section ?? '')) return false
    onPatch({ section: trimmed === '' ? null : trimmed })
  }

  // The cell holds the description's first line, and only that line travels
  // back: the paragraphs under it are the open row's to edit.
  const headline = (text: string | null) => {
    const next = withHeadline(scene.description, text ?? '')
    if (next === scene.description) return false
    onPatch({ description: next })
  }
  const more = linesAfterHeadline(scene.description)

  const readiness = readinessOf(scene, vocabulary.scene_blocks, frames)
  const chosen = chosenFrame(frames)

  return (
    <TableRow id={`scene-${scene.id}`} className="align-middle">
      {/* The hint sits on the cell rather than on the field: InlineField
          takes no title of its own, and the thumbnail beside it has one. */}
      <TableCell className={cn(CELL, 'pr-1')} title={t('scenes.moveToHint')}>
        <span className="flex items-center justify-end gap-1.5">
          {/* The frame the scene is cut from, beside its number: the board
              answers "which scene is this picture" and "which picture is
              this scene" in the same glance - what the predecessor could
              not do. A thumbnail rather than a column, so a board without
              pictures costs no width. */}
          {chosen && (
            <Button
              variant="icon"
              size="icon-sm"
              onClick={() => onViewFrame(chosen)}
              title={chosen.original_name ?? t('scenes.openFrame')}
              aria-label={t('scenes.openFrame')}
              className="overflow-hidden"
            >
              <img src={fileSrc(chosen.path)} alt="" className="size-7 object-contain" />
            </Button>
          )}
          {/* The number is where the board is reordered from, because the
              number IS the order: typing 3 on scene 12 says "this one
              happens third" in the same place the person is already reading
              the order. A drag would be the other way to say it, and it is
              the wrong way on a board you scroll - dragging 12 to 3 on a
              fifty-scene table means holding the mouse while the rows crawl
              past. Typing works whether the target is on screen or forty
              rows away.

              The board closes up behind the move: the rest are renumbered
              1..N, never nudged, so a board with holes or twins comes out
              of it straight (decision of 2026-09-16). Escape puts back what
              is stored: the field is a command, and a command is cancelled,
              not half-entered. The column's heading names the cells, so
              each field keeps its caption for a screen reader only. */}
          <InlineField
            className="w-8 [&_input]:text-right"
            label={t('scenes.moveTo')}
            labelHidden
            codec={numberCodec}
            value={scene.position}
            onCommit={moveTo}
            disabled={moving}
          />
        </span>
      </TableCell>

      <TableCell className={CELL}>
        <InlineField
          className="w-24"
          label={t('scenes.section')}
          labelHidden
          placeholder={t('scenes.sectionPlaceholder')}
          value={scene.section}
          onCommit={section}
        />
      </TableCell>

      <TableCell className={CELL}>
        <span className="flex items-center gap-1">
          {/* Against the dash, so the span reads as one "0:14 – 0:42". */}
          <InlineField
            className="w-14 [&_input]:text-right"
            label={t('scenes.startsAt')}
            labelHidden
            placeholder="0:00"
            codec={timecodeCodec}
            value={scene.starts_at}
            onCommit={timecode('starts_at', scene.starts_at)}
          />
          <span aria-hidden className="text-faint">
            –
          </span>
          <InlineField
            className="w-14"
            label={t('scenes.endsAt')}
            labelHidden
            placeholder="0:00"
            codec={timecodeCodec}
            value={scene.ends_at}
            onCommit={timecode('ends_at', scene.ends_at)}
          />
        </span>
      </TableCell>

      {vocabulary.shot_types.length > 0 && (
        <TableCell className={CELL}>
          {/* A pick from the kind's words rather than text, but dressed as
              the cells beside it: no box at rest, the same hairline under
              the pointer. An unset kind reads faint, like an empty cell. */}
          <Select
            className={cn(
              'h-control-sm w-24 -ml-1.5 rounded-sm border-transparent px-1.5 py-0',
              'hover:border-line-2 hover:bg-soft data-[popup-open]:border-accent',
              scene.shot_type === null && 'text-faint',
            )}
            aria-label={t('scenes.shotType')}
            value={scene.shot_type ?? ''}
            placeholder={t('scenes.noShot')}
            onChange={(value) => onPatch({ shot_type: value === '' ? null : value })}
            options={vocabulary.shot_types.map((shot) => ({
              value: shot.key,
              label: sayLabel(shot.label),
            }))}
          />
        </TableCell>
      )}

      <TableCell className={CELL}>
        <span className="flex min-w-40 items-center gap-1">
          {/* The description's paragraphs, the prompt blocks and the
              pictures open under the row: a table of fifty scenes is
              readable because the long text is folded away, not because it
              was left out. */}
          <Button
            variant="icon"
            size="icon-sm"
            aria-expanded={open}
            aria-label={t(open ? 'scenes.fold' : 'scenes.unfold', { number: scene.position })}
            title={t(open ? 'scenes.fold' : 'scenes.unfold', { number: scene.position })}
            onClick={onToggle}
          >
            <ChevronRight
              aria-hidden
              className={cn('text-faint transition-transform', open && 'rotate-90')}
            />
          </Button>
          <InlineField
            className="flex-1"
            label={t('scenes.description')}
            labelHidden
            placeholder={t('scenes.describeIt')}
            value={headlineOf(scene.description)}
            onCommit={headline}
          />
          {/* The cell shows one line; this says there is more under it. */}
          {more > 0 && (
            <span
              className="shrink-0 text-xs text-faint tabular-nums"
              title={t('scenes.moreLines', { count: more })}
            >
              +{more}
            </span>
          )}
          {/* Here rather than in the last column: the description gives way
              to it, where a cell of its own would have widened the column
              and moved the whole board on every save. */}
          {saving && <Spinner size="sm" tone="dim" label={t('save.saving')} />}
        </span>
      </TableCell>

      {aboutShown && (
        // Who and where, as names rather than as text inside the shot: a
        // hero written three ways across fifty scenes is three heroes to
        // anything that reads them. Chosen in the open row.
        <TableCell className={CELL}>
          <span className="flex flex-wrap gap-1">
            {about.length === 0 ? (
              <span className="text-xs text-faint">—</span>
            ) : (
              about.map((link) => (
                // Held to a width, so a long name costs the description a
                // few letters rather than a column.
                <Chip key={link.note_id} className="max-w-28" title={link.note_title ?? undefined}>
                  <span className="truncate">{link.note_title ?? t('scenes.untitledNote')}</span>
                </Chip>
              ))
            )}
          </span>
        </TableCell>
      )}

      <TableCell className={CELL}>
        <ReadinessMark readiness={readiness} blocks={vocabulary.scene_blocks.length} />
      </TableCell>

      {/* Not a right click as well, the way the catalogue's rows have one:
          every cell here is a text field, and inside a field the right click
          is the only way to cut, copy and paste with the mouse. */}
      <TableCell className="px-1 text-right">
        <RowMenu label={t('scenes.rowMenu', { number: scene.position })} actions={actions} />
      </TableCell>
    </TableRow>
  )
}
