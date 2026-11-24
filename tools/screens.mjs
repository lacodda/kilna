// The screens of the documentation, taken from a build.
//
//   node tools/screens.mjs --check
//   node tools/screens.mjs --app <kilna binary> [--out docs/public/screens]
//                          [--only <name>] [--native-driver <path>]
//
// docs/screens.json lists every picture a page of the documentation shows:
// where in the window it is taken and what has to be on the screen first.
// The script makes a demo stand with the build's own `--demo-all`, opens the
// build's window on it through tauri-driver and saves each picture as
// `<out>/<name>.png`. Nothing it writes is kept in git: the site is built
// from the build it documents (the Docs workflow), so a picture cannot fall
// behind the program, and one that can no longer be taken stops the site.
//
// `--check` needs no build: it holds the list and the pages to each other.
//
// The driver is spoken to in plain WebDriver over HTTP - a session, a script,
// a click, a screenshot - which is all this needs of a client library.

import { spawn, spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const MANIFEST = join(root, 'docs', 'screens.json')
const PAGES = join(root, 'docs', 'src', 'content', 'docs')
const README = join(root, 'README.md')
/// Where the site serves the pictures from, and the README reads them from.
const SERVED = '/kilna/screens/'
const SITE = 'https://lacodda.github.io'
const PORT = 4444
const DRIVER = `http://127.0.0.1:${PORT}`
/// How long one thing may take to come to the screen.
const PATIENCE = 20_000

function fail(message) {
  console.error(`screens: ${message}`)
  process.exit(1)
}

function option(name) {
  const at = process.argv.indexOf(`--${name}`)
  return at < 0 ? undefined : process.argv[at + 1]
}

/// The list, with what is wrong in it said all at once.
function manifest() {
  const { size, storage, screens } = JSON.parse(readFileSync(MANIFEST, 'utf8'))
  const problems = []
  const seen = new Set()
  for (const screen of screens) {
    const name = screen.name ?? '(unnamed)'
    if (!/^[a-z0-9-]+$/.test(name))
      problems.push(`${name}: a name is lowercase words joined by hyphens`)
    if (seen.has(name)) problems.push(`${name}: listed twice`)
    seen.add(name)
    if (!screen.alt || screen.alt.length < 20)
      problems.push(`${name}: say in \`alt\` what the picture shows`)
    if (!Array.isArray(screen.steps) || screen.steps.length === 0)
      problems.push(`${name}: no steps`)
    for (const step of screen.steps ?? []) {
      const kinds = Object.keys(step)
      if (
        kinds.length !== 1 ||
        !['open', 'click', 'wait', 'keys', 'type', 'gone'].includes(kinds[0])
      )
        problems.push(
          `${name}: a step is one of open, click, wait, gone, keys, type - not ${JSON.stringify(step)}`,
        )
    }
  }
  if (problems.length > 0) fail(`docs/screens.json\n  ${problems.join('\n  ')}`)
  return { size, storage, screens }
}

function markdownFiles(directory) {
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry)
    if (statSync(path).isDirectory()) return markdownFiles(path)
    return /\.mdx?$/.test(entry) ? [path] : []
  })
}

/// Every picture the pages and the README show, with where and under which words.
function shown() {
  const found = []
  const image = /!\[([^\]]*)\]\(([^)\s]+)\)|<img\s[^>]*src="([^"]+)"[^>]*alt="([^"]*)"/g
  for (const path of [...markdownFiles(PAGES), README]) {
    const text = readFileSync(path, 'utf8')
    for (const match of text.matchAll(image)) {
      const address = match[2] ?? match[3]
      const alt = match[1] ?? match[4]
      const local = address.startsWith(SITE) ? address.slice(SITE.length) : address
      if (!local.startsWith(SERVED)) continue
      found.push({
        path: path.slice(root.length + 1).replaceAll('\\', '/'),
        name: local.slice(SERVED.length).replace(/\.png$/, ''),
        alt,
      })
    }
  }
  return found
}

/// The list and the pages agree: every picture shown is taken, every picture
/// taken is shown, and a page describes it in the words the list does.
function check(screens) {
  const problems = []
  const pictures = shown()
  const listed = new Map(screens.map((screen) => [screen.name, screen]))
  for (const picture of pictures) {
    const screen = listed.get(picture.name)
    if (!screen)
      problems.push(
        `${picture.path} shows ${picture.name}.png, which docs/screens.json does not take`,
      )
    else if (screen.alt !== picture.alt)
      problems.push(
        `${picture.path} describes ${picture.name}.png in other words than docs/screens.json`,
      )
  }
  const used = new Set(pictures.map((picture) => picture.name))
  for (const screen of screens) {
    if (!used.has(screen.name)) problems.push(`${screen.name}.png is taken and no page shows it`)
  }
  if (problems.length > 0) fail(`the pages and the list disagree\n  ${problems.join('\n  ')}`)
  return pictures.length
}

async function ask(method, path, body) {
  const response = await fetch(`${DRIVER}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const answer = await response.json()
  if (!response.ok) {
    const error = new Error(`${method} ${path}: ${answer.value?.message ?? response.status}`)
    error.code = answer.value?.error
    throw error
  }
  return answer.value
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms))

async function until(what, probe) {
  const deadline = Date.now() + PATIENCE
  let refused = ''
  for (;;) {
    const value = await probe().catch((cause) => {
      refused = ` (${cause.message})`
    })
    if (value) return value
    if (Date.now() > deadline)
      throw new Error(`${what} did not come in ${PATIENCE / 1000} s${refused}`)
    await sleep(150)
  }
}

/// What a step points at, found in the page: `"words"` is something that
/// reads exactly so - its own text, or the name it carries for a reader - and
/// `{ "text": "words", "in": "css" }` the same inside a part of the screen;
/// `{ "css": "..." }` is a selector. Only what is on the screen counts.
const FIND = `
  const [target] = arguments
  const shown = (element) => {
    const box = element.getBoundingClientRect()
    return box.width > 0 && box.height > 0 && getComputedStyle(element).visibility !== 'hidden'
  }
  if (target.css && !target.text) return [...document.querySelectorAll(target.css)].find(shown) ?? null
  const own = (element) =>
    [...element.childNodes]
      .filter((node) => node.nodeType === Node.TEXT_NODE)
      .map((node) => node.textContent.trim())
      .filter(Boolean)
      .join(' ')
  for (const element of document.body.querySelectorAll('*')) {
    if (target.in && !element.closest(target.in)) continue
    const reads =
      own(element) === target.text ||
      element.getAttribute('aria-label') === target.text ||
      element.getAttribute('title') === target.text
    if (reads && shown(element)) return element
  }
  return null
`

/// The key WebDriver hands an element back under.
const ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'

class Window {
  constructor(session) {
    this.at = `/session/${session}`
  }

  script(source, ...args) {
    return ask('POST', `${this.at}/execute/sync`, { script: source, args })
  }

  async find(target) {
    if (typeof target === 'string') target = { text: target }
    return until(
      `${JSON.stringify(target)}`,
      async () => (await this.script(FIND, target))?.[ELEMENT],
    )
  }

  async gone(target) {
    if (typeof target === 'string') target = { text: target }
    await until(
      `${JSON.stringify(target)} to leave`,
      async () => (await this.script(FIND, target)) === null,
    )
  }

  /// What the window remembers between runs - its language, its theme - set
  /// before the first picture, so every machine takes the same one.
  async remember(storage) {
    await this.find({ css: 'a[href="/catalogue"]' })
    await this.script(
      `for (const [key, value] of Object.entries(arguments[0])) localStorage.setItem(key, value)`,
      storage,
    )
    await ask('POST', `${this.at}/refresh`, {})
    await this.find({ css: 'a[href="/catalogue"]' })
  }

  /// A route of the window, taken the way a link takes it: the router hears
  /// the history move, and nothing is loaded again.
  async open(path) {
    await this.script(
      `history.pushState({}, '', arguments[0]); dispatchEvent(new PopStateEvent('popstate'))`,
      path,
    )
  }

  /// A screen draws itself again as its data arrives, and what was found a
  /// moment ago may be gone by the press: found and pressed again until it
  /// takes.
  async click(target) {
    await until(`a press on ${JSON.stringify(target)}`, async () => {
      const id = await this.find(target)
      await ask('POST', `${this.at}/element/${id}/click`, {})
      return true
    })
  }

  /// Keys as a person presses them: `Control+k`, `Escape`.
  async keys(chord) {
    const names = { Control: '', Shift: '', Alt: '', Escape: '', Enter: '', Tab: '' }
    const keys = chord.split('+').map((key) => names[key] ?? key)
    const actions = [
      ...keys.map((value) => ({ type: 'keyDown', value })),
      ...keys.toReversed().map((value) => ({ type: 'keyUp', value })),
    ]
    await ask('POST', `${this.at}/actions`, { actions: [{ type: 'key', id: 'keyboard', actions }] })
    await ask('DELETE', `${this.at}/actions`)
  }

  async type(text) {
    const actions = [...text].flatMap((value) => [
      { type: 'keyDown', value },
      { type: 'keyUp', value },
    ])
    await ask('POST', `${this.at}/actions`, { actions: [{ type: 'key', id: 'keyboard', actions }] })
    await ask('DELETE', `${this.at}/actions`)
  }

  /// The screen has stopped moving: its fonts are in, nothing is being
  /// fetched, and two looks a moment apart see the same page.
  async settled() {
    await this.script(`return document.fonts.ready.then(() => true)`)
    let before = ''
    await until('the screen to settle', async () => {
      const now = await this.script(
        `return document.body.innerText.length + ':' + document.querySelectorAll('*').length + ':' + document.getAnimations().filter((a) => a.playState === 'running' && a.effect?.getTiming().iterations !== Infinity).length`,
      )
      const still = now === before && now.endsWith(':0')
      before = now
      return still
    })
  }

  async picture() {
    return Buffer.from(await ask('GET', `${this.at}/screenshot`), 'base64')
  }
}

function run(app, args) {
  const made = spawnSync(app, args, { encoding: 'utf8' })
  if (made.status !== 0) fail(`${app} ${args.join(' ')}\n${made.stderr || made.stdout}`)
  return made.stdout
}

async function take({ size, storage, screens }) {
  const app = resolve(option('app') ?? fail('--app <kilna binary> is needed to take pictures'))
  const out = resolve(option('out') ?? join(root, 'docs', 'public', 'screens'))
  const only = option('only')
  const stand = mkdtempSync(join(tmpdir(), 'kilna-screens-'))
  run(app, ['--demo-all', '--workspace', stand])

  const native = option('native-driver')
  const driver = spawn(
    'tauri-driver',
    ['--port', String(PORT), ...(native ? ['--native-driver', native] : [])],
    {
      stdio: ['ignore', 'inherit', 'inherit'],
    },
  )
  driver.on('error', (cause) => fail(`tauri-driver would not start: ${cause.message}`))
  let session
  try {
    await until('tauri-driver', () => fetch(`${DRIVER}/status`).then((answer) => answer.ok))
    const made = await ask('POST', '/session', {
      capabilities: {
        alwaysMatch: { 'tauri:options': { application: app, args: [`--workspace=${stand}`] } },
      },
    })
    session = made.sessionId
    const window = new Window(session)
    await ask('POST', `${window.at}/window/rect`, { width: size.width, height: size.height })
    await window.remember(storage ?? {})

    mkdirSync(out, { recursive: true })
    for (const screen of screens) {
      if (only && screen.name !== only) continue
      try {
        for (const step of screen.steps) {
          if (step.open) await window.open(step.open)
          else if (step.click) await window.click(step.click)
          else if (step.wait) await window.find(step.wait)
          else if (step.gone) await window.gone(step.gone)
          else if (step.keys) await window.keys(step.keys)
          else if (step.type) await window.type(step.type)
        }
        await window.settled()
      } catch (cause) {
        writeFileSync(
          join(out, `${screen.name}.failed.png`),
          await window.picture().catch(() => Buffer.alloc(0)),
        )
        throw new Error(`${screen.name}: ${cause.message}`, { cause })
      }
      writeFileSync(join(out, `${screen.name}.png`), await window.picture())
      console.log(`took  ${screen.name}.png`)
      // Whatever a picture opened - a palette, a menu - is closed for the next.
      await window.keys('Escape').catch(() => undefined)
    }
  } finally {
    if (session) await ask('DELETE', `/session/${session}`).catch(() => undefined)
    driver.kill()
    rmSync(stand, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 })
  }
}

const listed = manifest()
const { screens } = listed
const pictures = check(screens)
if (process.argv.includes('--check')) {
  console.log(`screens: ${screens.length} pictures listed, shown ${pictures} times`)
} else {
  await take(listed).catch((cause) => fail(cause.message))
}
