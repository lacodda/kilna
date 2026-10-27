import type { TFunction } from 'i18next'

/*
 * A kind's folder on disk, as the profile names it (ADR 0057): a path under
 * the media folder, with `{title}`, a field's `{key}`, or either read off what
 * the work is made from after `origin.`.
 *
 * The same rules the backend holds the profile to (`folder::problem`), checked
 * as the template is typed so the box says what is wrong before the bar does.
 */

/** A character no folder name may hold on Windows, where the rules are the
 *  tightest: a template written on one machine names folders on all of them. */
// eslint-disable-next-line no-control-regex
const FORBIDDEN = /[<>:"|?*\u0000-\u001f]/

/** What is wrong with a folder template, or `null` when it can name a folder
 *  (an empty one removes it). */
export function folderProblem(text: string, t: TFunction): string | null {
  const template = text.trim()
  if (template === '') return null
  if (/^[\\/]/.test(template) || /^[A-Za-z]:/.test(template)) return t('editor.folderAbsolute')
  for (const raw of template.split(/[\\/]/)) {
    const segment = raw.trim()
    if (segment === '') return t('editor.folderEmptyStep')
    if (segment === '.' || segment === '..') return t('editor.folderClimbs')
    let rest = segment
    for (;;) {
      const open = rest.indexOf('{')
      const words = open === -1 ? rest : rest.slice(0, open)
      if (words.includes('}')) return t('editor.folderBraces')
      if (FORBIDDEN.test(words)) return t('editor.folderForbidden', { text: words })
      if (open === -1) break
      const close = rest.indexOf('}', open)
      if (close === -1) return t('editor.folderBraces')
      const name = rest.slice(open + 1, close)
      if (!/^(origin\.)?[A-Za-z0-9_-]+$/.test(name)) {
        return t('editor.folderPlaceholder', { name: `{${name}}` })
      }
      rest = rest.slice(close + 1)
    }
  }
  return null
}

/** A character no file name may hold, the stars and marks of a rule aside. */
// eslint-disable-next-line no-control-regex
const FORBIDDEN_IN_RULE = /[<>:"|\u0000-\u001f]/

/**
 * What is wrong with a kind's rule for its finished file (ADR 0070), or
 * `null` when it can name one (an empty one removes it): a path inside the
 * work's folder whose steps may hold `*` and `?`, as `video/*.mp4`. The
 * backend's `folder::rule_problem`, checked as it is typed.
 */
export function fileRuleProblem(text: string, t: TFunction): string | null {
  const rule = text.trim()
  if (rule === '') return null
  if (/^[\\/]/.test(rule) || /^[A-Za-z]:/.test(rule)) return t('editor.fileAbsolute')
  for (const step of rule.split(/[\\/]/)) {
    if (step === '') return t('editor.folderEmptyStep')
    if (step === '.' || step === '..') return t('editor.fileClimbs')
    if (FORBIDDEN_IN_RULE.test(step)) return t('editor.fileForbidden', { text: step })
  }
  return null
}
