/**
 * Checklists in a markdown body: `- [ ]` and `- [x]`.
 *
 * The rendered body draws them as checkboxes, and ticking one rewrites the
 * text — the body stays the one truth, and the checkbox is only a way of
 * editing it. The boxes are matched to lines by order: the n-th checkbox the
 * renderer drew is the n-th task line here, which holds as long as both count
 * the same lines. So lines inside a fenced code block are skipped, exactly as
 * the renderer skips them.
 */

/** A task line: optional quote markers, a bullet or a number, then the box. */
const TASK = /^((?:[ \t]*>)*[ \t]*(?:[-*+]|\d+[.)])[ \t]+\[)([ xX])(\](?:[ \t]|$))/

/** The opening or closing fence of a code block. */
const FENCE = /^(?:[ \t]*>)*[ \t]{0,3}(`{3,}|~{3,})/

/** Where the task lines are, in order, and whether each is ticked. */
export function tasksOf(body: string): { line: number; done: boolean }[] {
  const found: { line: number; done: boolean }[] = []
  let fence: string | null = null
  body.split('\n').forEach((text, line) => {
    const opener = FENCE.exec(text)
    if (opener !== null) {
      const mark = opener[1] ?? ''
      // A fence closes on the same character, at least as long as it opened.
      if (fence === null) fence = mark
      else if (mark.startsWith(fence.charAt(0)) && mark.length >= fence.length) fence = null
      return
    }
    if (fence !== null) return
    const task = TASK.exec(text)
    if (task !== null) found.push({ line, done: task[2] !== ' ' })
  })
  return found
}

/** The body with its `index`-th task ticked or unticked. Out of range is the
 *  body unchanged: a checkbox that no longer has a line is not a reason to
 *  write anything. */
export function toggleTask(body: string, index: number): string {
  const task = tasksOf(body)[index]
  if (task === undefined) return body
  const lines = body.split('\n')
  lines[task.line] = (lines[task.line] ?? '').replace(TASK, (_, open: string, mark: string, close: string) =>
    `${open}${mark === ' ' ? 'x' : ' '}${close}`,
  )
  return lines.join('\n')
}

/** How many of the body's tasks are done, of how many. */
export function progressOf(body: string): { done: number; total: number } {
  const tasks = tasksOf(body)
  return { done: tasks.filter((task) => task.done).length, total: tasks.length }
}
