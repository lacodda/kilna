import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { Clapperboard, Square } from 'lucide-react'
import type { Missing, PluginReply, ShortView, Work } from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { cancelPlugin, onPluginProgress, runPlugin } from '@/lib/api/plugins'
import { nameOf } from '@/lib/media'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { formatSeconds } from '@/lib/timecode'
import { say } from '@/lib/toast'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { Frame } from '@/components/ui/frame'
import { Progress } from '@/components/ui/progress'
import { Loaded } from '@/components/Loaded'
import { ShortFrame } from '@/features/shorts/ShortFrame'
import { useShortLayers } from '@/features/shorts/useShortLayers'
import { useBuilder } from '@/features/work/tabs/build/useBuild'
import { useWorkEdit } from '@/features/work/tabs/overview/useWorkEdit'

/** The key of a work's fields its variant is kept under when a person gave
 *  it one (`short::VARIANT_KEY`). */
const VARIANT_KEY = 'short_variant'

/**
 * The Build tab of a short (v0.168): the frame it will wear, drawn live; the
 * variant of the space's style it wears - the round's by the day it goes
 * out, or one given by hand; what it still lacks; and the build itself, by
 * kilna-plugin-cut, with its progress, a way to stop it, and the file it
 * made.
 */
export function ShortBuildTab({ work }: { work: Work }) {
  const short = useQuery(queries.short(work.id))
  return (
    <Frame>
      <Loaded query={short}>{(view) => <Body work={work} view={view} />}</Loaded>
    </Frame>
  )
}

function Body({ work, view }: { work: Work; view: ShortView }) {
  const { t } = useTranslation()
  const builder = useBuilder()
  const stage = useShortLayers(work.id)
  const { setField } = useWorkEdit(work)

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

  const build = useAppMutation({
    mutationFn: async () => {
      if (builder === undefined) throw new Error(t('shorts.noBuilder'))
      const id = crypto.randomUUID()
      stopped.current = false
      setRunning({ run: id, share: null, message: t('shorts.drawing') })
      try {
        await stage()
        return await runPlugin(builder.executable, 'cut', 'cuts', work.id, id)
      } finally {
        setRunning(null)
      }
    },
    refresh: [...refresh.work, ...refresh.finishedFile(work.id), keys.shortFor(work.id)],
    onError: (cause) => {
      if (stopped.current) say.info(t('plugins.stopped'))
      else say.failed(cause)
    },
    onSuccess: (reply: PluginReply) => {
      if (reply.message !== null && reply.message.trim() !== '') say.info(reply.message)
      if (reply.finished !== null)
        say.ok(t('plugins.madeFinished', { name: nameOf(reply.finished) }))
    },
  })

  const stops = view.missing.filter((lack) => lack.what !== 'cover')
  const given = view.variant?.by_hand === true ? view.variant.key : ''

  return (
    <div className="flex min-h-0 flex-1 flex-wrap items-start gap-6 overflow-y-auto">
      <ShortFrame
        look={view.look}
        title={view.title}
        cover={view.cover}
        className="w-64 shrink-0"
      />

      <div className="flex min-w-72 flex-1 flex-col gap-4">
        <section aria-label={t('shorts.variant')} className="flex flex-col gap-1.5">
          <b className="text-sm font-semibold">{t('shorts.variant')}</b>
          {view.variants.length === 0 ? (
            <p className="text-xs text-dim">
              {t('shorts.styleHasNoVariants')}{' '}
              <NavLink className="text-accent hover:underline" to="/settings/shorts">
                {t('shorts.openStyle')}
              </NavLink>
            </p>
          ) : (
            <>
              <Select
                aria-label={t('shorts.variant')}
                className="w-64"
                value={given}
                placeholder={t('shorts.byCalendar')}
                options={view.variants.map((variant) => ({
                  value: variant.key,
                  label: variant.label,
                }))}
                onChange={(key) => setField(VARIANT_KEY, key === '' ? null : key)}
              />
              <p className="text-xs text-dim">
                {view.variant === null
                  ? null
                  : view.variant.by_hand
                    ? t('shorts.wornByHand', { name: view.variant.label })
                    : t('shorts.wornByCalendar', { name: view.variant.label })}{' '}
                <NavLink className="text-accent hover:underline" to="/settings/shorts">
                  {t('shorts.openStyle')}
                </NavLink>
              </p>
            </>
          )}
        </section>

        <section aria-label={t('shorts.lacks')} className="flex flex-col gap-1.5">
          <b className="text-sm font-semibold">{t('shorts.madeOf')}</b>
          <ul className="flex flex-col gap-1 text-xs text-dim">
            <li>{t('shorts.stretches', { seconds: formatSeconds(view.seconds) })}</li>
            <li>{sayWords(t, view)}</li>
            {view.missing.map((lack, at) => (
              <li key={at} className={lack.what === 'cover' ? 'text-warn' : 'text-bad'}>
                {sayMissing(t, lack)}
              </li>
            ))}
          </ul>
        </section>

        <div className="flex flex-wrap items-center gap-2">
          {running !== null ? (
            <>
              <Progress
                size="sm"
                className="min-w-40 flex-1"
                label={t('shorts.building')}
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
              disabled={build.isPending || builder === undefined || stops.length > 0}
              onClick={() => build.mutate()}
            >
              <Clapperboard aria-hidden />
              {t('shorts.build')}
            </Button>
          )}
          {builder === undefined && (
            <span className="text-xs text-dim">{t('shorts.noBuilder')}</span>
          )}
        </div>

        {view.file !== null && (
          <section aria-label={t('shorts.built')} className="flex flex-col gap-1.5">
            <b className="text-sm font-semibold">{t('shorts.built')}</b>
            {/* The file changes under one name's sibling, never under one
                name: a new build is a new address, and the player takes it. */}
            <video
              key={view.file}
              controls
              preload="metadata"
              src={fileSrc(view.file)}
              aria-label={t('shorts.builtFile', { name: nameOf(view.file) })}
              className="aspect-[9/16] w-64 rounded-lg border border-line bg-soft"
            />
            <span className="font-mono text-2xs text-faint">{nameOf(view.file)}</span>
          </section>
        )}
      </div>
    </div>
  )
}

type Say = ReturnType<typeof useTranslation>['t']

/** What a short lacks, in the person's words. */
function sayMissing(t: Say, lack: Missing): string {
  switch (lack.what) {
    case 'cuts':
      return t('shorts.missing.cuts')
    case 'source':
      return t('shorts.missing.source', { title: lack.title })
    case 'folder':
      return t('shorts.missing.folder')
    case 'cover':
      return t('shorts.missing.cover')
  }
}

/** Where the words of a short come from, or why it has none. */
function sayWords(t: Say, view: ShortView): string {
  switch (view.words.state) {
    case 'off':
      return t('shorts.words.off')
    case 'noTimings':
      return t('shorts.words.noTimings')
    case 'noText':
      return t('shorts.words.noText')
    case 'unreadable':
      return t('shorts.words.unreadable', { file: nameOf(view.words.file) })
    case 'found':
      return t('shorts.words.found', { count: view.words.lines, file: nameOf(view.words.file) })
  }
}
