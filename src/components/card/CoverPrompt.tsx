import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Copy } from 'lucide-react'
import { updateWork, type SceneBlock, type Work } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
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
 * A kind that names no parts draws nothing here at all — a song whose cover
 * is the album's has no prompt of its own to write.
 */
export function CoverPrompt({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const blocks = vocabularyOf(profile.config, work.kind).cover_blocks

  const save = useMutation({
    mutationFn: (cover: Record<string, string>) => updateWork(work.id, { cover }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: keys.work(work.id) })
      void client.invalidateQueries({ queryKey: keys.journal })
      say.ok(t('cover.saved'))
    },
    onError: (cause) => say.failed(cause),
  })

  if (blocks.length === 0) return null

  return (
    <Panel className="flex flex-col gap-3 p-4">
      <header>
        <h3 className="text-sm font-semibold">{t('cover.title')}</h3>
        <p className="mt-1 text-sm text-dim">{t('cover.hint')}</p>
      </header>

      {blocks.map((block) => (
        <Block
          key={block.key}
          block={block}
          value={work.cover[block.key] ?? ''}
          disabled={save.isPending}
          // The whole set travels, the way a scene's blocks do: the log's
          // `before` then holds the set as it was, and an undo puts it back.
          onCommit={(text) => save.mutate({ ...work.cover, [block.key]: text })}
        />
      ))}
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

  return (
    <section className="flex flex-col gap-1.5">
      <SectionLabel className="justify-between">
        <span>{sayLabel(block.label)}</span>
        <Button
          variant="ghost"
          size="icon-sm"
          title={t('cover.copy')}
          aria-label={t('cover.copy')}
          disabled={shown.trim() === ''}
          onClick={() => {
            navigator.clipboard.writeText(shown).then(
              () => say.ok(t('cover.copied')),
              (cause: unknown) => say.failed(cause),
            )
          }}
        >
          <Copy aria-hidden className="size-3.5" />
        </Button>
      </SectionLabel>
      <Textarea
        value={shown}
        rows={3}
        disabled={disabled}
        placeholder={block.hint ?? undefined}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => {
          if (text !== null && text !== value) onCommit(text)
          setText(null)
        }}
      />
    </section>
  )
}
