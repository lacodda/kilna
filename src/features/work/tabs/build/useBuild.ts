import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { JsonValue, Plugin } from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { queries } from '@/lib/query/queries'
import { peaksOf } from '@/features/work/tabs/build/timeline'

/** What the simple-video plugin calls itself in its manifest. */
const RENDERER = 'simple-video'

/** The renderer the Build tab works with: the simple-video plugin, on and
 *  able to run, offering its render. */
export type Renderer = { executable: string; settings: Readonly<Record<string, JsonValue>> }

/** The installed renderer, if any (v0.149). */
function rendererOf(plugins: readonly Plugin[]): Renderer | undefined {
  const found = plugins.find(
    (plugin) =>
      plugin.usable &&
      plugin.enabled &&
      plugin.manifest?.name === RENDERER &&
      plugin.manifest.commands.some((command) => command.key === 'render'),
  )
  return found === undefined
    ? undefined
    : { executable: found.executable, settings: found.settings }
}

/** The renderer, read off the plugins kilna lists. */
export function useRenderer(): Renderer | undefined {
  return useRendererState().renderer
}

/** The renderer, and whether the plugins have been read at all: until they
 *  are, an address naming the Build tab is trusted rather than bounced. */
export function useRendererState(): { renderer: Renderer | undefined; known: boolean } {
  const plugins = useQuery(queries.plugins())
  return { renderer: rendererOf(plugins.data ?? []), known: plugins.data !== undefined }
}

/** A track decoded for the Build tab: what plays, and what its waveform is
 *  drawn from. */
export type Decoded =
  | { state: 'none' }
  | { state: 'loading' }
  /** No Web Audio here: the timeline still works, without a waveform or a
   *  preview of the sound. */
  | { state: 'unsupported' }
  | { state: 'failed'; reason: string }
  | { state: 'ready'; buffer: AudioBuffer; peaks: number[]; peak: number; length: number }

/** How many bars a waveform is drawn with. */
const BARS = 600

/** The track at `path`, decoded once per path. */
export function useDecoded(path: string | null | undefined): Decoded {
  const [decoded, setDecoded] = useState<{ path: string | null; value: Decoded }>({
    path: null,
    value: { state: 'none' },
  })
  const wanted = path ?? null
  const supported = typeof AudioContext !== 'undefined'
  useEffect(() => {
    if (wanted === null || !supported) return
    let gone = false
    const context = new AudioContext()
    void (async () => {
      try {
        const bytes = await (await fetch(fileSrc(wanted))).arrayBuffer()
        const buffer = await context.decodeAudioData(bytes)
        const channels = Array.from({ length: buffer.numberOfChannels }, (_, at) =>
          buffer.getChannelData(at),
        )
        const peaks = peaksOf(channels, BARS)
        if (!gone) {
          setDecoded({
            path: wanted,
            value: {
              state: 'ready',
              buffer,
              peaks,
              peak: peaks.reduce((most, one) => Math.max(most, one), 0),
              length: buffer.duration,
            },
          })
        }
      } catch (cause) {
        if (!gone) {
          setDecoded({ path: wanted, value: { state: 'failed', reason: String(cause) } })
        }
      } finally {
        void context.close()
      }
    })()
    return () => {
      gone = true
    }
  }, [wanted, supported])
  if (wanted === null) return { state: 'none' }
  if (!supported) return { state: 'unsupported' }
  // Until the track at this path is decoded, it is being read.
  return decoded.path === wanted ? decoded.value : { state: 'loading' }
}
