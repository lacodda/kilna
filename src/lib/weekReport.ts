import type { Touched, WeekReport } from '@/lib/api/types'

/** The words a week's report is written in - the window's. */
export interface WeekWords {
  /** The heading, with the week's first and last day already in it. */
  title: string
  made: string
  versions: string
  scored: string
  released: string
  /** The line for what has no work to list: notes and trials. */
  besides: string
  /** What an empty section says. */
  nothing: string
}

/**
 * What a week came to, as Markdown (v0.105): a heading, then works made,
 * versions written, scores given and releases out, each work with how many
 * times when more than once, and a line for notes and trials. Read from the
 * operations log, so it holds whatever was done that week, read or not.
 */
export function weekMarkdown(report: WeekReport, words: WeekWords): string {
  const section = (heading: string, touched: readonly Touched[]) => [
    `## ${heading} (${touched.reduce((sum, work) => sum + work.count, 0)})`,
    '',
    ...(touched.length === 0
      ? [`- ${words.nothing}`]
      : touched.map((work) => `- ${work.title}${work.count > 1 ? ` × ${work.count}` : ''}`)),
    '',
  ]
  return [
    `# ${words.title}`,
    '',
    ...section(words.made, report.made),
    ...section(words.versions, report.versions),
    ...section(words.scored, report.scored),
    ...section(words.released, report.released),
    words.besides,
    '',
  ].join('\n')
}

/** The Monday of the week `day` (an ISO date) falls in. */
export function mondayOf(day: string): string {
  const [year, month, date] = day.split('-').map(Number) as [number, number, number]
  const at = new Date(Date.UTC(year, month - 1, date))
  const back = (at.getUTCDay() + 6) % 7
  at.setUTCDate(at.getUTCDate() - back)
  return at.toISOString().slice(0, 10)
}
