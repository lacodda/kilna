import { describe, expect, it } from 'vitest'
import {
  DEFAULT_LAYOUT,
  loadLayout,
  otherLayout,
  saveLayout,
  type CalendarLayout,
  type LayoutStore,
} from './calendarLayout'

describe('the remembered calendar layout', () => {
  const store = (initial?: string): LayoutStore & { value: string | null } => ({
    value: initial ?? null,
    getItem() {
      return this.value
    },
    setItem(_key, value) {
      this.value = value
    },
  })

  it('starts with the queue beside the month', () => {
    expect(loadLayout(store())).toBe(DEFAULT_LAYOUT)
    expect(DEFAULT_LAYOUT).toBe('queue')
  })

  it('comes back as it was left', () => {
    const kept = store()
    saveLayout('full', kept)
    expect(loadLayout(kept)).toBe('full')
  })

  it('falls back rather than trusting whatever is in storage', () => {
    // A layout this build has no case for would reach the grid as a class name
    // that does not exist, which is a screen with no columns at all.
    expect(loadLayout(store('wide'))).toBe(DEFAULT_LAYOUT)
    expect(loadLayout(store(''))).toBe(DEFAULT_LAYOUT)
    expect(loadLayout(store('{"layout":"full"}'))).toBe(DEFAULT_LAYOUT)
  })

  it('does not throw when storage refuses to write', () => {
    // Private windows and full disks both do this; forgetting the choice is the
    // right outcome, a broken screen is not.
    const refusing: LayoutStore = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota exceeded')
      },
    }
    expect(() => saveLayout('full', refusing)).not.toThrow()
  })

  it('does not throw when storage refuses to be read', () => {
    const refusing: LayoutStore = {
      getItem: () => {
        throw new Error('access denied')
      },
      setItem: () => {},
    }
    expect(loadLayout(refusing)).toBe(DEFAULT_LAYOUT)
  })
})

describe('otherLayout', () => {
  it('is the one the toggle switches to', () => {
    expect(otherLayout('queue')).toBe('full')
    expect(otherLayout('full')).toBe('queue')
  })

  it('returns to where it started after two switches', () => {
    for (const layout of ['queue', 'full'] as CalendarLayout[]) {
      expect(otherLayout(otherLayout(layout))).toBe(layout)
    }
  })
})
