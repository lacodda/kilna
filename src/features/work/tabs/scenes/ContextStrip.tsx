import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { queries } from '@/lib/query/queries'
import { proseOf } from '@/lib/scenes'
import { Button } from '@/components/ui/button'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { Markdown } from '@/components/Markdown'

/** The role that holds what every scene has in common: hero, palette, lens. */
export const CONTEXT_ROLE = 'context'

/**
 * What every scene shares — the hero, the palette, the lens — as a strip over
 * the board.
 *
 * It is read from the current version of the `context` role: a body with
 * revisions, read by the assistant like any other text, and so edited where
 * bodies are edited; the button opens that lane. Here it is two lines of
 * prose, because the board is what the tab is for and a panel of the whole
 * context stood between the person and it. When the two lines cut it short,
 * the rest opens in place, as written.
 */
export function ContextStrip({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  const versions = useQuery(queries.versions(workId))
  const current = (versions.data ?? []).find(
    (version) => version.role === CONTEXT_ROLE && version.is_current,
  )
  const body = useQuery({ ...queries.version(current?.id ?? ''), enabled: current !== undefined })
  const text = body.data?.body.trim() ?? ''

  const [whole, setWhole] = useState(false)
  // Whether the two lines cut the text short, measured rather than guessed
  // from its length: a long line in a wide window fits, and a short one
  // with three line breaks does not. A callback ref, because the paragraph
  // arrives with the body, after the first render.
  const [cut, setCut] = useState(false)
  const watching = useRef<ResizeObserver | null>(null)
  const measure = (line: HTMLParagraphElement | null) => {
    watching.current?.disconnect()
    watching.current = null
    if (line === null) return
    const check = () => setCut(line.scrollHeight > line.clientHeight + 1)
    check()
    const watch = new ResizeObserver(check)
    watch.observe(line)
    watching.current = watch
  }

  return (
    <Panel className="flex items-start gap-2.5 px-3 py-2.5">
      <div className="min-w-0 flex-1">
        <SectionLabel>{t('scenes.context')}</SectionLabel>
        {current === undefined && versions.data !== undefined && (
          <p className="mt-1 text-sm text-dim">{t('scenes.noContext')}</p>
        )}
        {text !== '' &&
          (whole ? (
            <Markdown body={text} className="mt-1 text-sm text-dim" />
          ) : (
            <p ref={measure} className="mt-1 line-clamp-2 text-sm leading-relaxed text-dim">
              {proseOf(text)}
            </p>
          ))}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {(cut || whole) && (
          <Button variant="link" className="text-xs" onClick={() => setWhole(!whole)}>
            {whole ? t('scenes.contextFold') : t('scenes.contextWhole')}
          </Button>
        )}
        <Button
          size="sm"
          onClick={() => void navigate(`/works/${workId}/versions?role=${CONTEXT_ROLE}`)}
        >
          {current === undefined ? t('scenes.writeContext') : t('scenes.editContext')}
        </Button>
      </div>
    </Panel>
  )
}
