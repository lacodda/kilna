import { describe, expect, it } from 'vitest'
import { mondayOf, weekMarkdown } from '@/lib/weekReport'

describe('a week as Markdown', () => {
  it('lists what was made by work, with how many times', () => {
    const text = weekMarkdown(
      {
        from: '2026-10-26',
        until: '2026-11-02',
        made: [{ work_id: 'w1', title: 'Tide', count: 1 }],
        versions: [
          { work_id: 'w1', title: 'Tide', count: 3 },
          { work_id: 'w2', title: 'Harbour', count: 1 },
        ],
        scored: [],
        released: [],
        notes: 2,
        trials: 0,
      },
      {
        title: 'The week of Oct 26 – Nov 1',
        made: 'Works made',
        versions: 'Versions written',
        scored: 'Scores given',
        released: 'Releases out',
        besides: 'Notes written: 2. Trials made: 0.',
        nothing: 'nothing',
      },
    )

    expect(text).toContain('# The week of Oct 26 – Nov 1')
    expect(text).toContain('## Versions written (4)\n\n- Tide × 3\n- Harbour\n')
    expect(text).toContain('## Scores given (0)\n\n- nothing\n')
    expect(text).toContain('Notes written: 2. Trials made: 0.')
  })

  it('starts a week on its Monday', () => {
    expect(mondayOf('2026-10-27')).toBe('2026-10-26')
    expect(mondayOf('2026-10-26')).toBe('2026-10-26')
    expect(mondayOf('2026-11-01')).toBe('2026-10-26')
  })
})
