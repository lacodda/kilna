import type { StyleBrick, StyleBrickPatch, StyleBrickStatus } from '@/lib/api/types'

/*
 * The rules between a style as it is stored and the style open in its editor,
 * which saves itself as it is typed into.
 *
 * Until v0.79 the editor was a dialog with a Save button, and the two only met
 * twice: when it opened, and when it was saved. An editor that writes as it
 * goes meets the stored brick all the time - its own writes come back, and so
 * do other people's: the assistant keeps a description it wrote, an undo puts
 * a name back. The rules for which side wins are here, pure, so they are
 * tested rather than discovered by typing:
 *
 * - **A field being typed into is the person's.** A value arriving from
 *   elsewhere is noted as the stored one and the box is left alone; the typing
 *   is written over it at the next pause.
 * - **A field nobody is typing into follows what is stored.** The description
 *   the assistant wrote appears in the open editor without reopening it.
 * - **Spelling is not a change.** What is stored is trimmed, what is typed is
 *   not - "Low sun " is on its way to "Low sun, long reflections" - so a field
 *   whose stored value means what the box says keeps the box as it is, or the
 *   space just typed would vanish from under the caret.
 * - **A write the backend would refuse is not sent.** A style needs a name and
 *   may not share one with another of its type; the name and the type are
 *   held back while it breaks either rule, so the description typed beside
 *   them still lands.
 */

/** A style as its editor holds it: every field as the box spells it. */
export interface StyleForm {
  type_key: string
  name: string
  description: string
  hint: string
  status: StyleBrickStatus
}

type Field = keyof StyleForm

const FIELDS: readonly Field[] = ['type_key', 'name', 'description', 'hint', 'status']

/** A stored brick, spelled the way its editor's boxes spell it. */
export function formOf(brick: StyleBrick): StyleForm {
  return {
    type_key: brick.type_key,
    name: brick.name,
    description: brick.description ?? '',
    hint: brick.hint ?? '',
    status: brick.status,
  }
}

/** A text field as it would be stored: trimmed, and nothing when blank. */
const stored = (text: string): string | null => {
  const trimmed = text.trim()
  return trimmed === '' ? null : trimmed
}

/** One field in the stored spelling, so two spellings of one value compare equal. */
function meaning(form: StyleForm, field: Field): string | null {
  switch (field) {
    case 'name':
      return form.name.trim()
    case 'description':
    case 'hint':
      return stored(form[field])
    default:
      return form[field]
  }
}

/** Why a name cannot be written, if it cannot. */
export type NameProblem = 'missing' | 'taken'

/**
 * What stops `name` being a style's name, among the names of the styles of
 * its type other than itself. The backend compares names exactly, and so does
 * this: "Dusk" and "dusk" are two styles.
 */
export function nameProblem(name: string, others: Iterable<string>): NameProblem | null {
  const trimmed = name.trim()
  if (trimmed === '') return 'missing'
  for (const other of others) if (other === trimmed) return 'taken'
  return null
}

/**
 * What the editor has to write: every field whose meaning differs from what
 * is stored, in the stored spelling. `null` when there is nothing to write.
 *
 * `keyHeld` keeps the type and the name out of the write, while the name is
 * one `nameProblem` refuses. The backend refuses the whole write for a bad
 * name, and a description should not wait on a name being finished. The type
 * is held with it because the two are one key - a name is unique within its
 * type - and a type written alone can collide as surely as a name.
 */
export function patchOf(
  form: StyleForm,
  base: StyleForm,
  { keyHeld = false }: { keyHeld?: boolean } = {},
): StyleBrickPatch | null {
  const patch: StyleBrickPatch = {}
  let any = false
  for (const field of FIELDS) {
    if (meaning(form, field) === meaning(base, field)) continue
    if (keyHeld && (field === 'name' || field === 'type_key')) continue
    any = true
    switch (field) {
      case 'type_key':
        patch.type_key = form.type_key
        break
      case 'name':
        patch.name = form.name.trim()
        break
      case 'description':
        patch.description = stored(form.description)
        break
      case 'hint':
        patch.hint = stored(form.hint)
        break
      case 'status':
        patch.status = form.status
        break
    }
  }
  return any ? patch : null
}

/**
 * Take in a stored brick that arrived - the answer to a write, a refetch, an
 * undo. `form` is what the boxes show, `base` the stored brick as last seen.
 *
 * A field with an edit of its own (the box means something other than
 * `base`) keeps the box; every other field takes the stored value when it
 * means something else than the box. Either way the stored value becomes the
 * new `base`, so the next write is measured against what is actually there.
 */
export function adopt(
  form: StyleForm,
  base: StyleForm,
  incoming: StyleForm,
): { form: StyleForm; base: StyleForm } {
  const next = { ...form }
  for (const field of FIELDS) {
    const edited = meaning(form, field) !== meaning(base, field)
    if (!edited && meaning(incoming, field) !== meaning(form, field)) {
      Object.assign(next, { [field]: incoming[field] })
    }
  }
  return { form: next, base: { ...incoming } }
}

/**
 * The name a style is made under before anyone names it: `untitled`, or the
 * first of `untitled 2`, `untitled 3`... that no style of its type has, since
 * two of one type may not share a name.
 */
export function untitledName(untitled: string, taken: Iterable<string>): string {
  const names = new Set(taken)
  if (!names.has(untitled)) return untitled
  for (let n = 2; ; n += 1) {
    const candidate = `${untitled} ${String(n)}`
    if (!names.has(candidate)) return candidate
  }
}
