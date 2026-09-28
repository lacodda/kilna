import { useTranslation } from 'react-i18next'
import type { SceneBlock } from '@/lib/api/types'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'
import { CopyButton } from '@/components/ui/copy-button'
import { useFieldDraft } from '@/components/ui/field-draft'
import { Textarea } from '@/components/ui/textarea'
import { ActionBar } from '@/features/assistant/ActionBar'

interface Props {
  block: SceneBlock
  text: string
  /** The work and scene this block belongs to — the scene actions are
      offered here aimed at this block alone. Absent for a block the profile
      no longer names: there is nothing to regenerate into. */
  workId?: string
  sceneId?: string
  /** Absent for a block the profile no longer names: read, not written. */
  onSave?: (text: string) => void
}

/**
 * One prompt block: a caption, a copy button, the box. Copying is the whole
 * point of a block — it goes into a generator as it is — so the button is on
 * every block, and the tick comes only after the clipboard confirms.
 */
export function BlockBox({ block, text, workId, sceneId, onSave }: Props) {
  const { t } = useTranslation()
  const draft = useFieldDraft(text, (typed) => onSave?.(typed), { multiline: true })

  return (
    <div className="group flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <span className="flex-1 caption">{sayLabel(block.label)}</span>
        {/* What is in the box, not what was last stored: a press on Copy is
            the first thing to take the focus from a block being written, and
            copying the stored text handed the generator the prompt from
            before the edit - under a message that said it was copied. The
            tick and its announcement are the button's; a refusal is a toast. */}
        <CopyButton
          value={draft.value}
          label={t('scenes.copy', { block: sayLabel(block.label) })}
          copiedLabel={t('scenes.copied', { block: sayLabel(block.label) })}
          title={t('scenes.copy', { block: sayLabel(block.label) })}
          disabled={draft.value === ''}
          onCopy={(ok) => {
            if (!ok) say.failed(t('scenes.copyFailed', { block: sayLabel(block.label) }))
          }}
        />
        {/* The profile's scene actions, aimed at this block: the animation
            rewritten without touching the still. The answer is held to this
            block, and what it brings is laid over the scene rather than put
            in its place. */}
        {workId !== undefined && sceneId !== undefined && (
          <ActionBar workId={workId} sceneId={sceneId} block={block.key} compact />
        )}
      </div>
      <Textarea
        autoResize
        maxRows={12}
        rows={3}
        readOnly={onSave === undefined}
        aria-label={sayLabel(block.label)}
        {...draft}
      />
      {block.hint !== undefined && block.hint !== null && (
        <span className="text-xs text-faint">{sayLabel(block.hint)}</span>
      )}
    </div>
  )
}
