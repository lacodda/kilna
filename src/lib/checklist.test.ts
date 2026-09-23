import { describe, expect, it } from 'vitest'
import { progressOf, tasksOf, toggleTask } from '@/lib/checklist'

describe('checklists', () => {
  const body = ['Idea', '', '- [ ] check the layer', '- [x] find the term', '* [ ] third'].join('\n')

  it('finds the task lines in order, with their state', () => {
    expect(tasksOf(body)).toEqual([
      { line: 2, done: false },
      { line: 3, done: true },
      { line: 4, done: false },
    ])
  })

  it('ticks and unticks the n-th task and leaves every other line alone', () => {
    const ticked = toggleTask(body, 0)
    expect(ticked.split('\n')[2]).toBe('- [x] check the layer')
    expect(ticked.split('\n').filter((_, at) => at !== 2)).toEqual(
      body.split('\n').filter((_, at) => at !== 2),
    )
    expect(toggleTask(body, 1).split('\n')[3]).toBe('- [ ] find the term')
  })

  it('does not count a task line inside a code block, as the renderer does not', () => {
    const fenced = ['```', '- [ ] not a task', '```', '- [ ] a task'].join('\n')
    expect(tasksOf(fenced)).toEqual([{ line: 3, done: false }])
    expect(toggleTask(fenced, 0).split('\n')[3]).toBe('- [x] a task')
    expect(toggleTask(fenced, 0).split('\n')[1]).toBe('- [ ] not a task')
  })

  it('reads numbered and quoted tasks, and ignores brackets that are not a box', () => {
    expect(tasksOf('1. [ ] first\n> - [X] quoted\n- [link](x)\n-[ ] no space')).toEqual([
      { line: 0, done: false },
      { line: 1, done: true },
    ])
  })

  it('leaves the body alone for a box that has no line any more', () => {
    expect(toggleTask(body, 7)).toBe(body)
  })

  it('counts progress', () => {
    expect(progressOf(body)).toEqual({ done: 1, total: 3 })
  })
})
