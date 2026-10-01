import { useTranslation } from 'react-i18next'
import type { Note, Scene, SceneFrame, SceneNote, ScenePatch } from '@/lib/api/types'
import { textMap, textOf } from '@/lib/json'
import { FRAME, ofKind, VIDEO } from '@/lib/scenes'
import type { Vocabulary } from '@/lib/useProfile'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { useFieldDraft } from '@/components/ui/field-draft'
import { Textarea } from '@/components/ui/textarea'
import { ActionBar } from '@/features/assistant/ActionBar'
import { BlockBox } from '@/features/work/tabs/scenes/BlockBox'
import { SceneFrames } from '@/features/work/tabs/scenes/SceneFrames'
import { SceneFraming } from '@/features/work/tabs/scenes/SceneFraming'

interface Props {
  scene: Scene
  workId: string
  vocabulary: Vocabulary
  /** How many columns the row above spans, so this one spans them all. */
  columns: number
  /** The width of the board's visible pane, when it has been measured: the
      open row is held to it rather than to the table's own width. */
  paneWidth: number | null
  /** The pictures and clips drawn for it, in order. */
  frames: SceneFrame[]
  /** The notes a scene can be about: its characters and its places. */
  cast: Note[]
  /** Who the scene is about now. */
  about: SceneNote[]
  onAttach: (noteId: string) => void
  onDetach: (noteId: string) => void
  onPatch: (changes: ScenePatch) => void
  onViewFrame: (frame: SceneFrame) => void
}

/**
 * What does not fit a row, open under it: the description in full, who the
 * scene is about, a box per prompt block with its own copy button, and the
 * pictures and clips drawn from them.
 *
 * The scene's own actions from the assistant open here too, beside the
 * prompts they rewrite. They stood on every row as a button each and are the
 * reason the row's column of buttons grew to eight.
 */
export function SceneDetail({
  scene,
  workId,
  vocabulary,
  columns,
  paneWidth,
  frames,
  cast,
  about,
  onAttach,
  onDetach,
  onPatch,
  onViewFrame,
}: Props) {
  const { t } = useTranslation()

  const description = useFieldDraft(
    scene.description,
    (text) => {
      if (text === scene.description) return false
      onPatch({ description: text })
    },
    { multiline: true },
  )

  const saveBlock = (key: string, text: string) => {
    const blocks: Record<string, string> = textMap(scene.blocks)
    if (text.trim() === '') delete blocks[key]
    else blocks[key] = text
    if (textOf(scene.blocks[key]) !== (blocks[key] ?? '')) onPatch({ blocks })
  }

  // Blocks the scene holds under keys the profile no longer names: shown,
  // not editable, so text is never hidden and never typed into blindly.
  const unlisted = Object.keys(scene.blocks).filter(
    (key) => !vocabulary.scene_blocks.some((block) => block.key === key),
  )

  const named = new Set(about.map((link) => link.note_id))
  // The candidates, and whoever the scene already names that is not one of
  // them - a note of a kind the board no longer offers, linked before - so
  // that it can still be let go of here rather than only seen in the row.
  const offered = [
    ...cast.map((note) => ({ id: note.id, title: note.title })),
    ...about
      .filter((link) => !cast.some((note) => note.id === link.note_id))
      .map((link) => ({ id: link.note_id, title: link.note_title })),
  ]

  return (
    <tr className="border-b border-line bg-softer">
      <td colSpan={columns} className="px-3 py-3">
        {/* The open row is read, not scrolled. The table is wider than the
            pane when the window is narrow, and a row that inherited that
            width put the last prompt block off the edge - measured. `sticky
            left-0` with the pane's own width holds the panel where the eye
            is while the table scrolls under it. */}
        <div
          data-scene-detail={scene.id}
          className="sticky left-0 flex flex-col gap-3"
          style={paneWidth === null ? undefined : { width: paneWidth - 24 }}
        >
          <div className="flex items-start gap-2">
            <Textarea
              className="flex-1"
              autoResize
              maxRows={8}
              rows={2}
              placeholder={t('scenes.descriptionPlaceholder')}
              aria-label={t('scenes.description')}
              {...description}
            />
            {/* The profile's scene actions - the prompts for this scene -
                start here and come back as a revision of it. */}
            <ActionBar
              workId={workId}
              sceneId={scene.id}
              menu
              hint={t('scenes.sceneActionsHint', { number: scene.position })}
            />
          </div>

          {offered.length > 0 && (
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-xs text-dim">{t('scenes.aboutHint')}</span>
              {/* One group for the whole cast: its value is every note the
                  scene names, and what changed says which one came or went. */}
              <ChipGroup
                multiple
                aria-label={t('scenes.about')}
                value={[...named]}
                onValueChange={(next) => {
                  const kept = new Set(next)
                  for (const id of kept) if (!named.has(id)) onAttach(id)
                  for (const id of named) if (!kept.has(id)) onDetach(id)
                }}
              >
                {offered.map((note) => (
                  <Chip key={note.id} value={note.id}>
                    {note.title ?? t('scenes.untitledNote')}
                  </Chip>
                ))}
              </ChipGroup>
            </div>
          )}

          {vocabulary.scene_blocks.length > 0 && (
            <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-3">
              {vocabulary.scene_blocks.map((block) => (
                <BlockBox
                  key={block.key}
                  block={block}
                  text={textOf(scene.blocks[block.key])}
                  workId={workId}
                  sceneId={scene.id}
                  onSave={(text) => saveBlock(block.key, text)}
                />
              ))}
            </div>
          )}

          {/* The built frame, under the blocks it is written around: a
              layout picked here wraps the picture block in the clip's
              style and the scene's characters. Only for a kind that marks
              which block is the picture. */}
          {vocabulary.scene_blocks.some((block) => block.picture === true) && (
            <SceneFraming scene={scene} onFraming={(framing) => onPatch({ framing })} />
          )}

          {unlisted.length > 0 && (
            <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-3">
              {unlisted.map((key) => (
                <BlockBox
                  key={key}
                  block={{ key, label: key, hint: t('scenes.unlistedBlock') }}
                  text={textOf(scene.blocks[key])}
                />
              ))}
            </div>
          )}

          {/* The pictures, under the prompts they came from: the prompt is
              copied out to a generator and the answer comes back here. The
              clips beside them, animated from the pictures - which is the
              order the work happens in, left to right. */}
          <div className="grid grid-cols-[repeat(auto-fit,minmax(280px,1fr))] gap-3">
            <SceneFrames
              workId={workId}
              sceneId={scene.id}
              number={scene.position}
              kind={FRAME}
              frames={ofKind(frames, FRAME)}
              onOpen={onViewFrame}
            />
            <SceneFrames
              workId={workId}
              sceneId={scene.id}
              number={scene.position}
              kind={VIDEO}
              frames={ofKind(frames, VIDEO)}
              onOpen={onViewFrame}
            />
          </div>
        </div>
      </td>
    </tr>
  )
}
