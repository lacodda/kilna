import { describe, expect, it } from 'vitest'
import {
  forget,
  loadRecent,
  noteDeleted,
  noteOpened,
  RECENT_LIMIT,
  remember,
  saveRecent,
  type Recent,
  type RecentStore,
} from './recent'

const work = (id: string, title = `Work ${id}`): Recent => ({ id, title })

const store = (initial?: string): RecentStore & { value: string | null } => ({
  value: initial ?? null,
  getItem() {
    return this.value
  },
  setItem(_key, value) {
    this.value = value
  },
})

describe('remember', () => {
  it('puts a new work at the front', () => {
    expect(remember([work('a')], work('b'))).toEqual([work('b'), work('a')])
  })

  it('moves one already in the list rather than adding it twice', () => {
    const list = [work('a'), work('b'), work('c')]
    expect(remember(list, work('c'))).toEqual([work('c'), work('a'), work('b')])
  })

  it('takes the newer title when a work comes back renamed', () => {
    const list = [work('a', 'Old name')]
    expect(remember(list, work('a', 'New name'))).toEqual([work('a', 'New name')])
  })

  it('drops the oldest past the limit', () => {
    let list: Recent[] = []
    for (let n = 0; n < RECENT_LIMIT + 3; n += 1) list = remember(list, work(String(n)))

    expect(list).toHaveLength(RECENT_LIMIT)
    // The newest is first and the three oldest are gone.
    expect(list[0]?.id).toBe(String(RECENT_LIMIT + 2))
    expect(list.map((held) => held.id)).not.toContain('0')
  })

  it('does not change the list it was given', () => {
    const list = [work('a')]
    remember(list, work('b'))
    expect(list).toEqual([work('a')])
  })
})

describe('forget', () => {
  it('drops the work named', () => {
    expect(forget([work('a'), work('b')], 'a')).toEqual([work('b')])
  })

  it('leaves the list alone when the work is not in it', () => {
    expect(forget([work('a')], 'z')).toEqual([work('a')])
  })
})

describe('the stored list', () => {
  it('starts empty', () => {
    expect(loadRecent(store())).toEqual([])
  })

  it('comes back as it was left', () => {
    const list = [work('a'), work('b')]
    const kept = store()
    saveRecent(list, kept)
    expect(loadRecent(kept)).toEqual(list)
  })

  it('drops one malformed entry rather than the whole list', () => {
    const raw = JSON.stringify([work('a'), { id: 7 }, work('b'), null, { title: 'no id' }])
    expect(loadRecent(store(raw))).toEqual([work('a'), work('b')])
  })

  it('refuses a stored value that is not a list', () => {
    expect(loadRecent(store('not json at all'))).toEqual([])
    expect(loadRecent(store(JSON.stringify({ id: 'a', title: 'b' })))).toEqual([])
    expect(loadRecent(store(JSON.stringify(null)))).toEqual([])
  })

  it('refuses an entry with an empty id', () => {
    // It would render a link to `/works/`, which is the list, not a work.
    expect(loadRecent(store(JSON.stringify([{ id: '', title: 'Nowhere' }])))).toEqual([])
  })

  it('never returns more than the limit, whatever is in storage', () => {
    const many = Array.from({ length: RECENT_LIMIT + 5 }, (_, n) => work(String(n)))
    expect(loadRecent(store(JSON.stringify(many)))).toHaveLength(RECENT_LIMIT)
  })

  it('never writes more than the limit', () => {
    const many = Array.from({ length: RECENT_LIMIT + 5 }, (_, n) => work(String(n)))
    const kept = store()
    saveRecent(many, kept)
    expect(JSON.parse(kept.value ?? '[]')).toHaveLength(RECENT_LIMIT)
  })

  it('does not throw when storage refuses to write', () => {
    const refusing: RecentStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota exceeded')
      },
    }
    expect(() => saveRecent([work('a')], refusing)).not.toThrow()
  })
})

describe('noting what happened to a work', () => {
  it('puts an opened work at the front of what is stored', () => {
    const kept = store(JSON.stringify([work('a'), work('b')]))
    noteOpened(work('b'), kept)
    expect(loadRecent(kept)).toEqual([work('b'), work('a')])
  })

  it('reads storage each time rather than trusting a copy', () => {
    // Two cards opening in the same session must not overwrite each other.
    const kept = store()
    noteOpened(work('a'), kept)
    noteOpened(work('b'), kept)
    expect(loadRecent(kept)).toEqual([work('b'), work('a')])
  })

  it('drops a deleted work from what is stored', () => {
    const kept = store(JSON.stringify([work('a'), work('b')]))
    noteDeleted('a', kept)
    expect(loadRecent(kept)).toEqual([work('b')])
  })
})
