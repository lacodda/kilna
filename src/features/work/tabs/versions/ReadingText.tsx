import { useMemo, type Ref } from 'react'
import type { LineMark } from '@/components/ui/marked-text'
import { MarkedText } from '@/components/ui/marked-text'
import { Markdown } from '@/components/Markdown'
import { withSections } from '@/features/work/tabs/versions/metrics'

/** Where the text being read is: a selection inside it is a selection of the
 *  version, which is what "To the canon" hands on (`ToCanonButton`). */
export const READING_TEXT = 'data-reading-text'

interface Props {
  body: string
  /** Rendered as prose rather than set as typed. See `VersionRole.body`. */
  markdown: boolean
  /** Lines new since the version standing beside this one. */
  added: readonly LineMark[]
  /** The type the text is set in; see `textMetrics`. */
  metrics: string
  ref?: Ref<HTMLDivElement>
}

/**
 * A version, read.
 *
 * Only read. The whole body used to be a button into the editor, so a press
 * meant to select a line - to copy it, or just to hold the place - threw the
 * text into a full-screen editor instead, and there was no way to take one
 * line out of a lyric without a trip through the box. Writing is asked for
 * now: the pencil in the bar, or `E` (or Enter) once the text has the focus.
 *
 * A click still gives it the focus - it is focusable, never a tab stop - so
 * the key works right where the pointer left it; Tab reaches the scroller
 * around it instead, which is a stop whenever the text runs past the fold.
 */
export function ReadingText({ body, markdown, added, metrics, ref }: Props) {
  const lineMarks = useMemo(() => withSections(body, added), [body, added])

  return (
    // Marked as the text being read, so a selection made in it can be told
    // from one made anywhere else (`ToCanonButton`).
    <div ref={ref} tabIndex={-1} className="outline-none" {...{ [READING_TEXT]: '' }}>
      {markdown ? (
        <div className={metrics}>
          <Markdown body={body} />
        </div>
      ) : (
        <MarkedText text={body} lineMarks={lineMarks} className={metrics} />
      )}
    </div>
  )
}
