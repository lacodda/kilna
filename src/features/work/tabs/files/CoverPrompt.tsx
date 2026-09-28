import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { SceneBlock, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { textMap, textOf } from '@/lib/json'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { CopyButton } from '@/components/ui/copy-button'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  work: Work
}

/**
 * The prompt a work's cover picture is drawn from.
 *
 * A short is picked off a wall of thumbnails, so its cover is written, kept
 * and reworked the way the thing itself is. The parts — what to draw, what to
 * keep out, what words go on it — are the craft's, named in the profile's
 * `cover_blocks`, and each is edited and copied on its own because each goes
 * into a different field of whatever draws it.
 *
 * It sits on the Files tab, above the pictures. Writing the prompt and
 * looking at what came back are one activity; a tab of their own for each
 * would put them on opposite sides of the card.
 *
 * The parts stand side by side, not one under another. Three boxes the width
 * of the card stacked above the gallery took most of its height before the
 * first picture; in a row they are one panel the height of one box, which is
 * how the mockup draws the prompt, and each still keeps its own box and its
 * own copy button.
 *
 * A kind that names no parts draws nothing here at all — a song whose cover
 * is the album's has no prompt of its own to write.
 */
export function CoverPrompt({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const blocks = vocabularyOf(profile.config, work.kind).cover_blocks

  const save = useAppMutation({
    mutationFn: (cover: Record<string, string>) => updateWork(work.id, { cover }),
    refresh: [keys.work(work.id)],
    onSuccess: () => say.ok(t('cover.saved')),
  })

  if (blocks.length === 0) return null

  return (
    <Panel className="flex flex-col gap-2 px-3 py-2.5">
      <header className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
        <SectionLabel>{t('cover.title')}</SectionLabel>
        <p className="text-xs text-faint">{t('cover.hint')}</p>
      </header>

      <div className="grid grid-cols-[repeat(auto-fit,minmax(14rem,1fr))] gap-2.5">
        {blocks.map((block) => (
          <Block
            key={block.key}
            block={block}
            value={textOf(work.cover[block.key])}
            disabled={save.isPending}
            // The whole set travels, the way a scene's blocks do: the log's
            // `before` then holds the set as it was, and an undo puts it back.
            onCommit={(text) => save.mutate({ ...textMap(work.cover), [block.key]: text })}
          />
        ))}
      </div>
    </Panel>
  )
}

/**
 * One part of the prompt.
 *
 * Held locally while it is typed and committed on blur, for the reason the
 * cut's timecode field gives: a save per keystroke would write half-words and
 * redraw the box under the cursor.
 */
function Block({
  block,
  value,
  disabled,
  onCommit,
}: {
  block: SceneBlock
  value: string
  disabled: boolean
  onCommit: (text: string) => void
}) {
  const { t } = useTranslation()
  const [text, setText] = useState<string | null>(null)
  const shown = text ?? value
  const name = sayLabel(block.label)

  return (
    // `group`: the copy button shows while the pointer is over the part it
    // copies, and whenever the keyboard is on it.
    <section className="group flex min-w-0 flex-col gap-1">
      <div className="flex min-h-5 items-center justify-between gap-2">
        <span className="caption truncate">{name}</span>
        {/* What is in the box, not what is stored: pressing Copy is what
            takes the focus from a part being written, and its save has not
            landed by then. */}
        <CopyButton
          value={shown}
          label={t('cover.copy')}
          copiedLabel={t('cover.copied')}
          title={t('cover.copy')}
          disabled={shown.trim() === ''}
          onCopy={(ok) => {
            if (!ok) say.failed(t('work.copyFailed'))
          }}
        />
      </div>
      <Textarea
        value={shown}
        rows={3}
        disabled={disabled}
        aria-label={name}
        placeholder={sayLabel(block.hint) || undefined}
        className="font-mono text-xs leading-relaxed"
        onChange={(event) => setText(event.target.value)}
        onBlur={() => {
          if (text !== null && text !== value) onCommit(text)
          setText(null)
        }}
      />
    </section>
  )
}
