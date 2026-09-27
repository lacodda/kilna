import { mockConvertFileSrc, mockIPC, mockWindows } from '@tauri-apps/api/mocks'

export type Args = Record<string, unknown>
export type Handler = (args: Args) => unknown

export interface Call {
  cmd: string
  args: Args
}

/**
 * The backend a test talks to: every command the window sends goes through
 * here, and each is answered by a handler or refused.
 *
 * Refused out loud. A command nobody answers rejects, and is listed in
 * `unanswered`, so a screen that starts asking for something new fails its
 * smoke test by name rather than rendering an error state that happens to
 * look fine - the way a test goes green over a screen it never drew.
 */
export interface Backend {
  /** Every command the window sent, in order. */
  readonly calls: readonly Call[]
  /** Commands sent that nothing answers, in order, repeats kept. */
  readonly unanswered: readonly string[]
  /** Answer `cmd` with `handler` from now on, in place of whatever did. */
  answer(cmd: string, handler: Handler): void
  /** The arguments of every call to `cmd`, oldest first. */
  argsOf(cmd: string): Args[]
}

/**
 * The window's own plumbing: the title bar asks whether it is maximised and
 * listens for resizes. Answered here once rather than by every workspace.
 */
const WINDOW: Record<string, Handler> = {
  'plugin:window|is_maximized': () => false,
  'plugin:window|is_fullscreen': () => false,
  'plugin:window|is_focused': () => true,
  'plugin:window|show': () => null,
}

export function mockBackend(handlers: Record<string, Handler>): Backend {
  const table = new Map<string, Handler>(Object.entries({ ...WINDOW, ...handlers }))
  const calls: Call[] = []
  const unanswered: string[] = []

  mockWindows('main')
  mockConvertFileSrc('windows')
  mockIPC(
    (cmd, payload) => {
      const args = (payload ?? {}) as Args
      calls.push({ cmd, args })
      const handler = table.get(cmd)
      if (handler === undefined) {
        unanswered.push(cmd)
        return Promise.reject(new Error(`the test backend does not answer \`${cmd}\``))
      }
      // A structured copy, as the IPC boundary makes one: a component that
      // edits what it was given must not reach back into the fixture.
      return Promise.resolve(handler(args)).then((value) =>
        value === undefined ? null : structuredClone(value),
      )
    },
    { shouldMockEvents: true },
  )

  return {
    calls,
    unanswered,
    answer: (cmd, handler) => void table.set(cmd, handler),
    argsOf: (cmd) => calls.filter((call) => call.cmd === cmd).map((call) => call.args),
  }
}
