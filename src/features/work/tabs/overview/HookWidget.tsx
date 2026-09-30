import { Fragment } from 'react'
import type { Work } from '@/lib/api/types'
import { fieldsOf } from '@/lib/overview'
import { fieldsFor, say as sayLabel, useProfile } from '@/lib/useProfile'
import { MetaParagraph } from '@/features/work/tabs/overview/MetaInput'
import { Widget } from '@/features/work/tabs/overview/Widget'
import { useWorkEdit } from '@/features/work/tabs/overview/useWorkEdit'

/**
 * What the work is about, in the author's words: the profile's paragraph
 * fields - a premise, an idea - read as paragraphs and rewritten in place.
 *
 * The first field names the widget; each after it carries its own caption,
 * as the mockup draws the hook and the idea in one widget. There is no tab
 * to go to: these fields live here, which is why the widget has no "Open".
 */
export function HookWidget({ work }: { work: Work }) {
  const profile = useProfile()
  const { setField } = useWorkEdit(work)
  // The work's kind's paragraphs: a field naming kinds belongs to those alone.
  const { prose } = fieldsOf(fieldsFor(profile.config, work.kind))
  const [first, ...rest] = prose
  if (first === undefined) return null

  const box = (field: typeof first) => (
    <MetaParagraph
      label={sayLabel(field.label)}
      value={work.meta[field.key] ?? null}
      onCommit={(text) => setField(field.key, text)}
    />
  )

  return (
    <Widget caption={sayLabel(first.label)}>
      {box(first)}
      {rest.map((field) => (
        <Fragment key={field.key}>
          <h3 className="caption mt-0.5">{sayLabel(field.label)}</h3>
          {box(field)}
        </Fragment>
      ))}
    </Widget>
  )
}
