// The transcripts of the documentation, taken from a build.
//
//   node tools/transcripts.mjs --check
//   node tools/transcripts.mjs --app <kilna binary>
//
// A page marks a block whose words are the program's own:
//
//   <!-- transcript: demo -->
//   <!-- transcript: mcp catalogue {"kind":"song","limit":3} -->
//   <!-- transcript: mcp scores {"work":"Winter road"} lines=12 -->
//
// and the fenced block right under the mark is written again from the build:
// `demo` is what `kilna --demo-all` prints as it makes the stand, `mcp <tool>
// <arguments>` what that tool answers over `kilna --mcp` on the stand's
// Studio space. `lines=N` keeps the first N lines and says the rest was cut.
//
// The Docs workflow runs this before it builds the site, so the site shows
// what the build it documents says; the blocks kept in git are the last
// taking, there so a page reads whole in the repository too.
//
// `--check` needs no build: every mark is one this script can take, and has
// its block under it.

import { spawn, spawnSync } from 'node:child_process'
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const PAGES = join(root, 'docs', 'src', 'content', 'docs')
/// The space of the stand the tools answer from.
const SPACE = 'Demo - Studio'
/// The folder a transcript shows in place of wherever the stand was made.
const SHOWN_STAND = 'demo'

function fail(message) {
  console.error(`transcripts: ${message}`)
  process.exit(1)
}

function option(name) {
  const at = process.argv.indexOf(`--${name}`)
  return at < 0 ? undefined : process.argv[at + 1]
}

function markdownFiles(directory) {
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry)
    if (statSync(path).isDirectory()) return markdownFiles(path)
    return /\.mdx?$/.test(entry) ? [path] : []
  })
}

const MARK = /^<!-- transcript: (.+?) -->$/

/// What a mark asks for, or why it cannot be taken.
function parse(said) {
  let rest = said.trim()
  let lines
  const cut = rest.match(/\s+lines=(\d+)$/)
  if (cut) {
    lines = Number(cut[1])
    rest = rest.slice(0, cut.index)
  }
  if (rest === 'demo') return { kind: 'demo', lines }
  const call = rest.match(/^mcp ([a-z_]+)(?:\s+(\{.*\}))?$/)
  if (!call) return { problem: `\`${said}\` is neither \`demo\` nor \`mcp <tool> {arguments}\`` }
  try {
    return { kind: 'mcp', tool: call[1], args: call[2] ? JSON.parse(call[2]) : {}, lines }
  } catch (cause) {
    return { problem: `the arguments of \`${said}\` are not JSON: ${cause.message}` }
  }
}

/// Every mark of a page: what it asks, and the lines of the block under it.
function marks(path) {
  const lines = readFileSync(path, 'utf8').split('\n')
  const found = []
  const problems = []
  const shown = path.slice(root.length + 1).replaceAll('\\', '/')
  lines.forEach((line, at) => {
    const mark = line.match(MARK)
    if (!mark) {
      if (line.includes('<!-- transcript'))
        problems.push(
          `${shown}:${at + 1}: a mark is a line of its own, \`<!-- transcript: ... -->\``,
        )
      return
    }
    const asked = parse(mark[1])
    if (asked.problem) return problems.push(`${shown}:${at + 1}: ${asked.problem}`)
    const opens = at + 1
    const fence = lines[opens]?.match(/^(`{3,})/)?.[1]
    const closes = fence ? lines.findIndex((other, index) => index > opens && other === fence) : -1
    if (closes < 0) return problems.push(`${shown}:${at + 1}: no fenced block right under the mark`)
    found.push({ ...asked, opens, closes, where: `${shown}:${at + 1}` })
  })
  return { lines, found, problems }
}

/// `kilna --mcp` on the stand: asked one request at a time, answered in order.
class Server {
  constructor(app, stand) {
    this.child = spawn(app, ['--mcp', '--workspace', stand], { stdio: ['pipe', 'pipe', 'inherit'] })
    this.waiting = new Map()
    this.next = 1
    let buffered = ''
    this.child.stdout.setEncoding('utf8')
    this.child.stdout.on('data', (chunk) => {
      buffered += chunk
      for (;;) {
        const end = buffered.indexOf('\n')
        if (end < 0) break
        const line = buffered.slice(0, end).trim()
        buffered = buffered.slice(end + 1)
        if (!line) continue
        const message = JSON.parse(line)
        this.waiting.get(message.id)?.(message)
        this.waiting.delete(message.id)
      }
    })
  }

  request(method, params) {
    const id = this.next++
    const answered = new Promise((done, refuse) => {
      this.waiting.set(id, done)
      this.child.once('exit', () =>
        refuse(new Error(`kilna --mcp ended before it answered ${method}`)),
      )
    })
    this.child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`)
    return answered
  }

  async start() {
    await this.request('initialize', {
      protocolVersion: '2025-06-18',
      capabilities: {},
      clientInfo: { name: 'docs', version: '1' },
    })
    this.child.stdin.write(
      `${JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' })}\n`,
    )
  }

  /// What a tool says: its text, and a refusal is an error here - a page
  /// shows what works.
  async call(tool, args) {
    const answer = await this.request('tools/call', {
      name: tool,
      arguments: { space: SPACE, ...args },
    })
    if (answer.error) throw new Error(`${tool}: ${answer.error.message}`)
    const text = (answer.result.content ?? []).map((part) => part.text ?? '').join('\n')
    if (answer.result.isError) throw new Error(`${tool} refused: ${text}`)
    return text
  }

  /// Hang up and wait for it to go: until it has, it holds the stand's files.
  stop() {
    if (this.child.exitCode !== null) return Promise.resolve()
    const gone = new Promise((done) => this.child.once('exit', done))
    this.child.stdin.end()
    return gone
  }
}

function cut(text, lines) {
  const all = text.replace(/\s+$/, '').split('\n')
  if (!lines || all.length <= lines) return all
  return [...all.slice(0, lines), `… ${all.length - lines} more lines`]
}

async function take(pages) {
  const app = resolve(option('app') ?? fail('--app <kilna binary> is needed to take transcripts'))
  const stand = mkdtempSync(join(tmpdir(), 'kilna-transcripts-'))
  const made = spawnSync(app, ['--demo-all', '--workspace', stand], { encoding: 'utf8' })
  if (made.status !== 0) fail(`${app} --demo-all\n${made.stderr || made.stdout}`)
  const server = new Server(app, stand)
  let written = 0
  try {
    await server.start()
    for (const [path, page] of pages) {
      // From the last mark up, so a block that changes length moves nothing
      // still to be written.
      for (const mark of page.found.toReversed()) {
        let block
        if (mark.kind === 'demo') {
          block = [`$ kilna --demo-all --workspace ${SHOWN_STAND}`, ...cut(made.stdout, mark.lines)]
        } else {
          const text = await server
            .call(mark.tool, mark.args)
            .catch((cause) => fail(`${mark.where}: ${cause.message}`))
          block = cut(text, mark.lines)
        }
        page.lines.splice(mark.opens + 1, mark.closes - mark.opens - 1, ...block)
        written++
      }
      writeFileSync(path, page.lines.join('\n'))
    }
  } finally {
    await server.stop()
    rmSync(stand, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 })
  }
  console.log(`transcripts: ${written} taken from ${app}`)
}

const pages = new Map()
const problems = []
let count = 0
for (const path of markdownFiles(PAGES)) {
  const page = marks(path)
  problems.push(...page.problems)
  count += page.found.length
  if (page.found.length > 0) pages.set(path, page)
}
if (problems.length > 0) fail(`\n  ${problems.join('\n  ')}`)
if (count === 0) fail('no page marks a transcript')

if (process.argv.includes('--check')) {
  console.log(`transcripts: ${count} marked on ${pages.size} pages`)
} else {
  await take(pages)
}
