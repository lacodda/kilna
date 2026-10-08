import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { version } from '../../package.json'

/*
 * The two halves of the window's one wire, held to each other.
 *
 * The backend registers its commands in `generate_handler!`; the window calls
 * them by name through `invoke`. Nothing connects the names at compile time.
 * A command the window never calls is a capability built and never offered -
 * by 24.09 thirteen of them, among them the button that keeps an assistant's
 * description of a style. A name the window calls that the backend does not
 * register fails only when someone presses the button.
 *
 * A command may wait for its screen, but only by saying which version brings
 * it: the promise is written here, and this test fails once that version is
 * the one being built and the command is still waiting.
 */

const ROOT = fileURLToPath(new URL('../../', import.meta.url))

/** Commands with no caller yet, each with the version whose screen calls it. */
const AWAITING: Record<string, { version: string; screen: string }> = {}

function* files(dir: string, extension: RegExp): Generator<string> {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) yield* files(path, extension)
    else if (extension.test(name)) yield path
  }
}

/** Every command the backend registers, by the last segment of its path. */
function registered(): Set<string> {
  const lists = [...files(join(ROOT, 'src-tauri/src'), /\.rs$/)].flatMap((path) =>
    [...readFileSync(path, 'utf8').matchAll(/generate_handler!\[([^\]]*)\]/g)].map((m) => m[1]!),
  )
  expect(lists, 'one generate_handler! in the backend').toHaveLength(1)
  const names = lists[0]!
    .split(',')
    .map((entry) => entry.trim().split('::').at(-1)!)
    .filter((name) => name !== '')
  // The scan's watchdog: the backend registers well over a hundred.
  expect(names.length).toBeGreaterThan(100)
  return new Set(names)
}

/** Every command the window calls by name, wherever the call is written. */
function invoked(): Set<string> {
  const names = new Set<string>()
  for (const path of files(join(ROOT, 'src'), /\.tsx?$/)) {
    if (/\.test\.tsx?$/.test(path) || /[\\/]test[\\/]/.test(path)) continue
    for (const match of readFileSync(path, 'utf8').matchAll(/\binvoke(?:<[^(]*>)?\(\s*'(\w+)'/g)) {
      names.add(match[1]!)
    }
  }
  expect(names.size, 'commands the window calls').toBeGreaterThan(100)
  return names
}

/** Whether `a` is an earlier version than `b`. */
function before(a: string, b: string): boolean {
  const [x, y] = [a, b].map((v) => v.split('.').map(Number))
  for (let i = 0; i < 3; i += 1) {
    if (x![i]! !== y![i]!) return x![i]! < y![i]!
  }
  return false
}

describe('the wire between the window and the backend', () => {
  it('calls only commands the backend registers', () => {
    const backend = registered()
    const unknown = [...invoked()].filter((name) => !backend.has(name))
    expect(unknown, 'called but never registered').toEqual([])
  })

  it('calls every command the backend registers, or says which version will', () => {
    const window = invoked()
    const idle = [...registered()].filter((name) => !window.has(name) && !(name in AWAITING))
    expect(idle, 'registered, never called, and no version promised').toEqual([])
  })

  it('keeps every promise by the version it names', () => {
    const backend = registered()
    const window = invoked()
    for (const [name, promise] of Object.entries(AWAITING)) {
      expect(backend.has(name), `${name} is awaited but not registered`).toBe(true)
      expect(window.has(name), `${name} is called now; take it off the list`).toBe(false)
      expect(
        before(version, promise.version),
        `${name} was promised to ${promise.screen} by v${promise.version}, and this is v${version}`,
      ).toBe(true)
    }
  })
})
