/**
 * A release kind, shortened to sit beside a title without crushing it.
 *
 * The calendar chip carries a handle, the work's title, its readiness marks and
 * the kind. Spelled out, the kind won: on a day holding two releases the pilot
 * saw "SHORT" beside a title one letter wide. The word itself stays in the
 * chip's tooltip.
 *
 * Initials of the first two words ("Video clip" → VC), or the first two letters
 * of a single word ("Short" → SH). The profile writes these labels, so this is
 * a shortening rule rather than a lookup table: the code never gets to know
 * which kinds exist (ADR 0001).
 */
export function shortKind(label: string): string {
  const [first, second] = label.split(/[\s\-–—/]+/u).filter((word) => word !== '')
  if (first === undefined) return ''
  const short = second === undefined ? first.slice(0, 2) : first.slice(0, 1) + second.slice(0, 1)
  // Upper-cased here rather than left to `text-transform`: the chip is not the
  // only place a kind may be shortened, and a rule that lives in one stylesheet
  // is a rule the next caller does not get.
  return short.toLocaleUpperCase()
}
