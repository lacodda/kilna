import { useEffect, useRef, useState, type PointerEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Clapperboard, Gauge, Minus, Pause, Play, Plus, Square } from 'lucide-react'
import type { PluginReply, Work } from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { cancelPlugin, onPluginProgress, runPlugin } from '@/lib/api/plugins'
import { formatDelta } from '@/lib/format'
import { formatSeconds } from '@/lib/timecode'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Frame } from '@/components/ui/frame'
import { NumberField } from '@/components/ui/number-field'
import { Progress } from '@/components/ui/progress'
import { Slider } from '@/components/ui/slider'
import { useWorkEdit } from '@/features/work/tabs/overview/useWorkEdit'
import {
  clips,
  defaultsOf,
  GAIN_RANGE,
  pictureAt,
  planOf,
  soundAt,
  storedOf,
  timelineOf,
  TIMELINE_KEY,
  type Plan,
  type Timeline,
} from '@/features/work/tabs/build/timeline'
import { useDecoded, useRenderer, type Decoded } from '@/features/work/tabs/build/useBuild'

/** How long after the last change a timeline is written: one entry of the
 *  history for a drag or a number typed, not one a step. */
const SETTLE_MS = 700

/**
 * The Build tab (v0.149, FR-15): a work's simple video as two tracks - the
 * picture, V1, and the sound, A1 - with what of the track plays, its fades,
 * its gain, the picture held before and after it, a live preview of all of
 * it before anything is rendered, and the render itself by the simple-video
 * plugin, with its progress and a way to stop it.
 *
 * What is set is the work's `simple_video`, the shape the plugin renders by:
 * the preview here and the file the plugin makes follow one plan
 * (`planOf`). It is written as an ordinary edit of the work's fields, so undo
 * takes it back.
 */
export function BuildTab({ work }: { work: Work }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const renderer = useRenderer()
  const sources = useQuery(queries.buildSources(work.id))
  const { setField } = useWorkEdit(work)
  const decoded = useDecoded(sources.data?.track)
  const defaults = defaultsOf(renderer?.settings ?? {})

  // The timeline as kept, and the one being set: a change waits a moment
  // before it is written, and an undo or another window's edit is taken in
  // while nothing is waiting.
  const kept = timelineOf(work.meta)
  const keptKey = JSON.stringify(storedOf(kept))
  const [line, setLine] = useState<Timeline>(kept)
  const [seen, setSeen] = useState(keptKey)
  const [pending, setPending] = useState(false)
  if (keptKey !== seen) {
    setSeen(keptKey)
    if (!pending) setLine(kept)
  }
  const change = (patch: Timeline) => {
    setLine((now) => ({ ...now, ...patch }))
    setPending(true)
  }
  useEffect(() => {
    if (!pending) return
    const timer = setTimeout(() => {
      setField(TIMELINE_KEY, storedOf(line))
      setPending(false)
    }, SETTLE_MS)
    return () => clearTimeout(timer)
  }, [line, pending, setField])

  const length = decoded.state === 'ready' ? decoded.length : (line.trim_end ?? 0)
  const plan = planOf(line, length, defaults)

  // A run of the renderer: its progress as the plugin writes it, and a stop.
  const [running, setRunning] = useState<{
    run: string
    share: number | null
    message: string | null
  } | null>(null)
  const stopped = useRef(false)
  useEffect(() => {
    let stop: (() => void) | undefined
    let gone = false
    void onPluginProgress((step) => {
      setRunning((now) =>
        now?.run === step.run
          ? { ...now, share: step.progress ?? now.share, message: step.message ?? now.message }
          : now,
      )
    }).then((unlisten) => {
      if (gone) unlisten()
      else stop = unlisten
    })
    return () => {
      gone = true
      stop?.()
    }
  }, [])
  const run = useAppMutation({
    mutationFn: async (command: 'render' | 'analyze_audio') => {
      if (renderer === undefined) throw new Error(t('build.noRenderer'))
      const id = crypto.randomUUID()
      stopped.current = false
      setRunning({ run: id, share: null, message: null })
      try {
        return await runPlugin(renderer.executable, command, 'work', work.id, id)
      } finally {
        setRunning(null)
      }
    },
    refresh: [...refresh.work, ...refresh.finishedFile(work.id)],
    onError: (cause) => {
      if (stopped.current) say.info(t('plugins.stopped'))
      else say.failed(cause)
    },
    onSuccess: async (reply: PluginReply, command) => {
      if (reply.message !== null && reply.message.trim() !== '') say.info(reply.message)
      if (command !== 'analyze_audio') return
      // The measure is kept in the work's fields: its gain to the target is
      // the gain that brings the track there.
      const fresh = await client.fetchQuery({ ...queries.work(work.id), staleTime: 0 })
      const loudness = fresh.meta.audio_loudness
      const gain =
        loudness !== null && typeof loudness === 'object' && !Array.isArray(loudness)
          ? loudness.gain_to_target
          : undefined
      if (typeof gain === 'number') {
        change({ gain_db: Math.min(GAIN_RANGE.max, Math.max(GAIN_RANGE.min, gain)) })
      }
    },
  })

  if (renderer === undefined) {
    return (
      <Frame>
        <p className="text-sm text-dim">{t('build.noRenderer')}</p>
      </Frame>
    )
  }

  const picture = sources.data?.picture ?? null
  const track = sources.data?.track ?? null
  const clipping = decoded.state === 'ready' && clips(decoded.peak, plan.gainDb)
  // What applies when a box is empty: the plan's value, the settings' fades.
  const effective: Record<keyof Timeline, number> = {
    trim_start: plan.trimStart,
    trim_end: plan.trimStart + plan.sound,
    gain_db: plan.gainDb,
    audio_fade_in: plan.audioFadeIn,
    audio_fade_out: plan.audioFadeOut,
    video_fade_in: plan.videoFadeIn,
    video_fade_out: plan.videoFadeOut,
    lead_in: plan.leadIn,
    tail: plan.tail,
  }
  const seconds = (key: keyof Timeline, label: string, max?: number) => (
    <label className="flex flex-col gap-1 text-xs text-dim">
      {label}
      <NumberField
        aria-label={label}
        value={line[key] ?? null}
        placeholder={formatSeconds(effective[key])}
        min={0}
        max={max}
        step={0.1}
        unit={t('build.seconds')}
        className="w-28"
        onValueChange={(value) => change({ [key]: value ?? undefined })}
      />
    </label>
  )

  return (
    <Frame>
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
        <Preview
          plan={plan}
          picture={picture}
          looped={sources.data?.looped ?? false}
          decoded={decoded}
        />

        <section
          aria-label={t('build.v1')}
          className="flex flex-col gap-2 rounded-lg border border-line p-3"
        >
          <header className="flex items-center gap-2">
            <b className="text-sm font-semibold">{t('build.v1')}</b>
            <span className="min-w-0 truncate text-xs text-dim">
              {picture === null
                ? t('build.noPicture')
                : sources.data?.looped === true
                  ? t('build.loop')
                  : t('build.still')}
            </span>
            <span className="ml-auto font-mono text-xs text-dim">{formatSeconds(plan.total)}</span>
          </header>
          <VideoBar plan={plan} />
          <div className="flex flex-wrap gap-3">
            {seconds('lead_in', t('build.leadIn'))}
            {seconds('tail', t('build.tail'))}
            {seconds('video_fade_in', t('build.videoFadeIn'))}
            {seconds('video_fade_out', t('build.videoFadeOut'))}
          </div>
        </section>

        <section
          aria-label={t('build.a1')}
          className="flex flex-col gap-2 rounded-lg border border-line p-3"
        >
          <header className="flex items-center gap-2">
            <b className="text-sm font-semibold">{t('build.a1')}</b>
            <span className="min-w-0 truncate text-xs text-dim">
              {track === null ? t('build.noTrack') : (sources.data?.track_name ?? track)}
            </span>
            <span className="ml-auto font-mono text-xs text-dim">{formatSeconds(plan.sound)}</span>
          </header>
          <Waveform decoded={decoded} plan={plan} onTrim={(patch) => change(patch)} />
          <div className="flex flex-wrap items-end gap-3">
            {seconds('trim_start', t('build.trimStart'), length || undefined)}
            {seconds('trim_end', t('build.trimEnd'), length || undefined)}
            {seconds('audio_fade_in', t('build.audioFadeIn'))}
            {seconds('audio_fade_out', t('build.audioFadeOut'))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-xs text-dim">{t('build.gain')}</span>
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('build.quieter')}
              onClick={() => change({ gain_db: Math.max(GAIN_RANGE.min, plan.gainDb - 1) })}
            >
              <Minus aria-hidden />
            </Button>
            <Slider
              aria-label={t('build.gain')}
              className="w-48"
              min={GAIN_RANGE.min}
              max={GAIN_RANGE.max}
              step={0.5}
              value={plan.gainDb}
              onValueChange={(value) =>
                change({ gain_db: typeof value === 'number' ? value : (value[0] ?? 0) })
              }
            />
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('build.louder')}
              onClick={() => change({ gain_db: Math.min(GAIN_RANGE.max, plan.gainDb + 1) })}
            >
              <Plus aria-hidden />
            </Button>
            <span
              className={clipping ? 'font-mono text-xs text-bad' : 'font-mono text-xs text-dim'}
            >
              {t('build.gainValue', { db: formatDelta(plan.gainDb, 1) })}
            </span>
            {clipping && <span className="text-xs text-bad">{t('build.clips')}</span>}
            <Button
              size="sm"
              variant="ghost"
              disabled={run.isPending || track === null}
              onClick={() => run.mutate('analyze_audio')}
            >
              <Gauge aria-hidden />
              {t('build.optimal')}
            </Button>
          </div>
        </section>

        <div className="flex flex-wrap items-center gap-2">
          {running !== null ? (
            <>
              <Progress
                size="sm"
                className="min-w-40 flex-1"
                label={t('build.rendering')}
                value={running.share === null ? null : running.share * 100}
              />
              {running.message !== null && (
                <span className="max-w-64 truncate text-xs text-dim">{running.message}</span>
              )}
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  stopped.current = true
                  cancelPlugin(running.run).catch((cause: unknown) => say.failed(cause))
                }}
              >
                <Square aria-hidden />
                {t('plugins.stop')}
              </Button>
            </>
          ) : (
            <Button
              variant="primary"
              disabled={run.isPending || picture === null || track === null}
              onClick={() => {
                // What is waiting to be written goes with the render.
                if (pending) {
                  setField(TIMELINE_KEY, storedOf(line))
                  setPending(false)
                }
                run.mutate('render')
              }}
            >
              <Clapperboard aria-hidden />
              {t('build.render')}
            </Button>
          )}
          {pending && <span className="text-xs text-faint">{t('save.saving')}</span>}
        </div>
      </div>
    </Frame>
  )
}

/** The picture's track: the hold before the sound, the sound, the hold after. */
function VideoBar({ plan }: { plan: Plan }) {
  const { t } = useTranslation()
  const share = (seconds: number) =>
    plan.total > 0 ? `${String((seconds / plan.total) * 100)}%` : '0%'
  return (
    <div
      className="flex h-6 w-full overflow-hidden rounded-md bg-soft"
      role="img"
      aria-label={t('build.v1Bar', {
        lead: formatSeconds(plan.leadIn),
        sound: formatSeconds(plan.sound),
        tail: formatSeconds(plan.tail),
      })}
    >
      <span className="h-full bg-line" style={{ width: share(plan.leadIn) }} />
      <span className="h-full bg-accent-soft" style={{ width: share(plan.sound) }} />
      <span className="h-full bg-line" style={{ width: share(plan.tail) }} />
    </div>
  )
}

const WAVE_HEIGHT = 72
/** How near an edge of the trim a press takes that edge, in pixels. */
const GRAB = 8

/** The track's waveform, what plays lit and the rest dimmed, with the edges
 *  of the trim dragged by the pointer. */
function Waveform({
  decoded,
  plan,
  onTrim,
}: {
  decoded: Decoded
  plan: Plan
  onTrim: (patch: Timeline) => void
}) {
  const { t } = useTranslation()
  const canvas = useRef<HTMLCanvasElement>(null)
  const dragging = useRef<'start' | 'end' | null>(null)
  const ready = decoded.state === 'ready' ? decoded : null

  useEffect(() => {
    const element = canvas.current
    const context = element?.getContext('2d')
    if (element === null || context === null || context === undefined || ready === null) return
    const width = element.clientWidth
    element.width = width
    element.height = WAVE_HEIGHT
    const styles = getComputedStyle(element)
    const lit = styles.getPropertyValue('--color-accent').trim() || 'currentColor'
    const dim = styles.getPropertyValue('--color-line-2').trim() || 'gray'
    const from = (plan.trimStart / ready.length) * width
    const to = ((plan.trimStart + plan.sound) / ready.length) * width
    context.clearRect(0, 0, width, WAVE_HEIGHT)
    const bar = width / ready.peaks.length
    ready.peaks.forEach((peak, at) => {
      const x = at * bar
      const h = Math.max(1, peak * (WAVE_HEIGHT - 4))
      context.fillStyle = x >= from && x <= to ? lit : dim
      context.fillRect(x, (WAVE_HEIGHT - h) / 2, Math.max(1, bar - 0.5), h)
    })
    context.fillStyle = lit
    context.fillRect(from - 1, 0, 2, WAVE_HEIGHT)
    context.fillRect(to - 1, 0, 2, WAVE_HEIGHT)
  }, [ready, plan.trimStart, plan.sound])

  if (decoded.state !== 'ready') {
    return (
      <p className="flex h-18 items-center text-xs text-faint">
        {decoded.state === 'loading'
          ? t('build.decoding')
          : decoded.state === 'unsupported'
            ? t('build.noWaveform')
            : decoded.state === 'failed'
              ? t('build.cannotDecode')
              : t('build.noTrack')}
      </p>
    )
  }

  const secondAt = (event: PointerEvent<HTMLCanvasElement>) => {
    const box = event.currentTarget.getBoundingClientRect()
    const share = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width))
    return Math.round(share * decoded.length * 10) / 10
  }
  const edgeAt = (event: PointerEvent<HTMLCanvasElement>) => {
    const box = event.currentTarget.getBoundingClientRect()
    const x = event.clientX - box.left
    const from = (plan.trimStart / decoded.length) * box.width
    const to = ((plan.trimStart + plan.sound) / decoded.length) * box.width
    if (Math.abs(x - from) <= GRAB) return 'start'
    if (Math.abs(x - to) <= GRAB) return 'end'
    return null
  }

  return (
    <canvas
      ref={canvas}
      role="img"
      aria-label={t('build.waveform', {
        from: formatSeconds(plan.trimStart),
        to: formatSeconds(plan.trimStart + plan.sound),
      })}
      className="h-18 w-full cursor-ew-resize rounded-md bg-soft"
      onPointerDown={(event) => {
        dragging.current = edgeAt(event)
        if (dragging.current !== null) event.currentTarget.setPointerCapture(event.pointerId)
      }}
      onPointerMove={(event) => {
        if (dragging.current === null) return
        const at = secondAt(event)
        if (dragging.current === 'start') {
          onTrim({ trim_start: Math.min(at, plan.trimStart + plan.sound - 0.5) })
        } else {
          onTrim({ trim_end: Math.max(at, plan.trimStart + 0.5) })
        }
      }}
      onPointerUp={() => {
        dragging.current = null
      }}
    />
  )
}

/** The video before it is rendered: the picture with its fades on a canvas,
 *  the sound with its trim, gain and fades through Web Audio, played and
 *  stopped by the button or the space bar. */
function Preview({
  plan,
  picture,
  looped,
  decoded,
}: {
  plan: Plan
  picture: string | null
  looped: boolean
  decoded: Decoded
}) {
  const { t } = useTranslation()
  const canvas = useRef<HTMLCanvasElement>(null)
  const still = useRef<HTMLImageElement>(null)
  const loop = useRef<HTMLVideoElement>(null)
  const [at, setAt] = useState(0)
  const [playing, setPlaying] = useState(false)
  const audio = useRef<{
    context: AudioContext
    source: AudioBufferSourceNode
    gain: GainNode
  } | null>(null)
  const started = useRef({ wall: 0, from: 0 })

  // One frame drawn: the picture fitted into 16:9 over black, as bright as
  // the fades make it at this second.
  const draw = (second: number) => {
    const context = canvas.current?.getContext('2d')
    if (context === null || context === undefined) return
    const { width, height } = context.canvas
    context.globalAlpha = 1
    // The frame's ground is the black the render pads with, by its token.
    const black = getComputedStyle(context.canvas).getPropertyValue('--color-black').trim()
    if (black !== '') context.fillStyle = black
    context.fillRect(0, 0, width, height)
    const source = looped ? loop.current : still.current
    const natural =
      source instanceof HTMLVideoElement
        ? { w: source.videoWidth, h: source.videoHeight }
        : source instanceof HTMLImageElement
          ? { w: source.naturalWidth, h: source.naturalHeight }
          : { w: 0, h: 0 }
    if (source === null || natural.w === 0) return
    const scale = Math.min(width / natural.w, height / natural.h)
    const w = natural.w * scale
    const h = natural.h * scale
    context.globalAlpha = pictureAt(plan, second)
    context.drawImage(source, (width - w) / 2, (height - h) / 2, w, h)
    context.globalAlpha = 1
  }

  const stop = () => {
    audio.current?.source.stop()
    void audio.current?.context.close()
    audio.current = null
    loop.current?.pause()
    setPlaying(false)
  }

  const play = () => {
    const from = at >= plan.total ? 0 : at
    started.current = { wall: performance.now(), from }
    if (decoded.state === 'ready' && typeof AudioContext !== 'undefined') {
      const context = new AudioContext()
      const source = context.createBufferSource()
      source.buffer = decoded.buffer
      const gain = context.createGain()
      gain.gain.value = soundAt(plan, from)
      source.connect(gain).connect(context.destination)
      const into = from - plan.leadIn
      if (into < plan.sound) {
        const wait = Math.max(0, -into)
        const offset = plan.trimStart + Math.max(0, into)
        source.start(context.currentTime + wait, offset, plan.sound - Math.max(0, into))
      }
      audio.current = { context, source, gain }
    }
    void loop.current?.play()
    setAt(from)
    setPlaying(true)
  }

  // While it plays, the clock is the wall's: the frame and the sound's level
  // follow it, and it stops at the end.
  useEffect(() => {
    if (!playing) {
      draw(at)
      return
    }
    let frame = 0
    const tick = () => {
      const second = started.current.from + (performance.now() - started.current.wall) / 1000
      if (second >= plan.total) {
        setAt(plan.total)
        stop()
        return
      }
      if (audio.current !== null) audio.current.gain.gain.value = soundAt(plan, second)
      draw(second)
      setAt(second)
      frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
    // `draw` and `stop` read the latest plan through the closure each frame.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing, plan.total])

  useEffect(() => () => stop(), [])

  // The space bar plays and stops, unless a box is being typed in.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      if (event.code !== 'Space' || target?.closest('input, textarea, [contenteditable]')) return
      event.preventDefault()
      if (playing) stop()
      else play()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  return (
    <section aria-label={t('build.preview')} className="flex flex-col gap-2">
      <canvas
        ref={canvas}
        width={640}
        height={360}
        className="aspect-video w-full max-w-xl rounded-lg"
        role="img"
        aria-label={t('build.preview')}
      />
      {picture !== null &&
        (looped ? (
          <video
            ref={loop}
            src={fileSrc(picture)}
            muted
            loop
            playsInline
            className="hidden"
            onLoadedData={() => draw(at)}
          />
        ) : (
          <img
            ref={still}
            src={fileSrc(picture)}
            alt=""
            className="hidden"
            onLoad={() => draw(at)}
          />
        ))}
      <div className="flex max-w-xl items-center gap-2">
        <Button
          variant="icon"
          size="icon-sm"
          aria-label={playing ? t('build.pause') : t('build.play')}
          disabled={plan.total <= 0}
          onClick={() => (playing ? stop() : play())}
        >
          {playing ? <Pause aria-hidden /> : <Play aria-hidden />}
        </Button>
        <Slider
          aria-label={t('build.scrub')}
          className="flex-1"
          min={0}
          max={Math.max(plan.total, 0.1)}
          step={0.1}
          value={Math.min(at, plan.total)}
          onValueChange={(value) => {
            if (playing) stop()
            const second = typeof value === 'number' ? value : (value[0] ?? 0)
            setAt(second)
            draw(second)
          }}
        />
        <span className="font-mono text-xs text-dim">
          {formatSeconds(at)} / {formatSeconds(plan.total)}
        </span>
      </div>
    </section>
  )
}
