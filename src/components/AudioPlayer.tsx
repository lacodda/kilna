import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Slider } from '@base-ui/react/slider'
import { ChevronDown, Pause, Play } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'

/** One thing the player can play: a file on disk and how it is named. */
export interface Track {
  /** Where it is, whole: what the window loads it by. */
  path: string
  /** What it is called on screen. */
  name: string
  /** Where it lies, said dim beside the name: a folder, or "attached". */
  where: string
}

/**
 * A player for the sounds of one thing: a button, the name, a bar to scrub
 * along, and the time.
 *
 * The browser's own controls were the other way, and they draw in the
 * system's colours at a width of their own; this is the same `<audio>`
 * underneath with the line's tokens on top. The bar is base-ui's slider, so
 * the arrows, Home and End and a press anywhere along it work as a slider's
 * do. dowel has no slider yet - asked for (v0.93).
 *
 * `selected` and `onSelect` are the caller's, so that something else on the
 * screen - a file's tile - can choose what plays; `request` is a counter the
 * caller bumps to say "play it now", which a choice alone does not.
 */
export function AudioPlayer({
  tracks,
  selected,
  onSelect,
  request,
  className,
}: {
  tracks: Track[]
  selected: string
  onSelect: (path: string) => void
  request: number
  className?: string
}) {
  const { t } = useTranslation()
  const audio = useRef<HTMLAudioElement>(null)
  const [playing, setPlaying] = useState(false)
  const [time, setTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [broken, setBroken] = useState(false)

  const track = tracks.find((one) => one.path === selected) ?? tracks[0]

  const path = track?.path

  // Asked to play: whatever is selected now, from where it stands. The
  // request is a count rather than a flag so that asking twice for the same
  // track plays it twice.
  useEffect(() => {
    if (request === 0) return
    const element = audio.current
    if (element === null) return
    // Refused or interrupted - a new source loading under it. The state
    // follows the element's own events, so nothing is claimed here.
    start(element).catch(() => undefined)
  }, [request, path])

  if (track === undefined) return null

  const toggle = () => {
    const element = audio.current
    if (element === null) return
    if (element.paused) {
      start(element).catch(() => setBroken(true))
    } else {
      element.pause()
    }
  }

  return (
    <div className={cn('flex min-w-0 items-center gap-2', className)}>
      <audio
        ref={audio}
        src={fileSrc(track.path)}
        preload="metadata"
        // A new track starts from its beginning and says nothing until it
        // loads: the element announces the new source itself.
        onLoadStart={() => {
          setTime(0)
          setDuration(0)
          setBroken(false)
        }}
        onPlay={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
        onEnded={() => setPlaying(false)}
        onTimeUpdate={(event) => setTime(event.currentTarget.currentTime)}
        onDurationChange={(event) => {
          const length = event.currentTarget.duration
          setDuration(Number.isFinite(length) ? length : 0)
        }}
        onError={() => {
          setBroken(true)
          setPlaying(false)
        }}
      />

      <Button
        size="icon-sm"
        variant={playing ? 'primary' : 'soft'}
        aria-label={playing ? t('player.pause') : t('player.play', { name: track.name })}
        title={playing ? t('player.pause') : t('player.play', { name: track.name })}
        disabled={broken}
        onClick={toggle}
      >
        {playing ? <Pause aria-hidden /> : <Play aria-hidden />}
      </Button>

      {tracks.length > 1 ? (
        <Menu>
          <MenuTrigger
            render={
              <Button
                size="sm"
                variant="icon"
                className="max-w-64 min-w-0 text-text"
                title={t('player.choose')}
              />
            }
          >
            <span className="truncate">{track.name}</span>
            <ChevronDown aria-hidden />
          </MenuTrigger>
          <MenuPopup align="start">
            {tracks.map((one) => (
              <MenuItem key={one.path} onClick={() => onSelect(one.path)}>
                <span className={cn('truncate', one.path === track.path && 'font-semibold')}>
                  {one.name}
                </span>
                <span className="ml-auto truncate pl-3 text-xs text-faint">{one.where}</span>
              </MenuItem>
            ))}
          </MenuPopup>
        </Menu>
      ) : (
        <span className="max-w-64 min-w-0 truncate px-1 text-sm" title={track.where}>
          {track.name}
        </span>
      )}

      {broken ? (
        <span className="text-xs text-bad">{t('player.broken')}</span>
      ) : (
        <>
          <Slider.Root
            value={time}
            min={0}
            max={duration > 0 ? duration : 1}
            step={0.1}
            disabled={duration === 0}
            onValueChange={(next) => {
              const element = audio.current
              if (element === null) return
              element.currentTime = next
              setTime(next)
            }}
            className="min-w-24 flex-1"
          >
            <Slider.Control className="flex h-4 w-full touch-none items-center select-none data-disabled:opacity-50">
              <Slider.Track className="relative h-1 w-full rounded-full bg-soft">
                <Slider.Indicator className="rounded-full bg-accent" />
                <Slider.Thumb
                  getAriaLabel={() => t('player.position')}
                  getAriaValueText={(_, value) =>
                    t('player.at', { time: clock(value), length: clock(duration) })
                  }
                  className={cn(
                    'size-3 rounded-full bg-accent shadow-lift',
                    'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent',
                  )}
                />
              </Slider.Track>
            </Slider.Control>
          </Slider.Root>
          <span className="shrink-0 font-mono text-xs text-dim tabular-nums">
            {clock(time)} / {duration > 0 ? clock(duration) : '–:––'}
          </span>
        </>
      )}
    </div>
  )
}

/** Start an element playing. `play()` answers with a promise in every
 *  browser kilna runs in, and with nothing in an older one - or in a test's
 *  document - so the answer is made a promise either way. */
function start(element: HTMLMediaElement): Promise<void> {
  try {
    return Promise.resolve(element.play())
  } catch (cause) {
    return Promise.reject(cause instanceof Error ? cause : new Error(String(cause)))
  }
}

/** Seconds as a clock reads them: `3:07`, or `1:02:45` past the hour. */
export function clock(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds))
  const hours = Math.floor(whole / 3600)
  const minutes = Math.floor((whole % 3600) / 60)
  const rest = String(whole % 60).padStart(2, '0')
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`
}
