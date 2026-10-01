import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { AlertTriangle } from 'lucide-react'
import type { Framing, Scene } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { CopyButton } from '@/components/ui/copy-button'
import { FramingPicker } from '@/features/cover/FramingPicker'
import { SchemeView } from '@/features/cover/SchemeView'

interface Props {
  scene: Scene
  onFraming: (framing: Framing | null) => void
}

/**
 * A scene's built frame (v0.88): the cover constructor's frame without the
 * words - where the hero stands, how big, how much of them shows - and the
 * still written around the scene's picture block in the clip's style, with
 * the characters the scene is about as its heroes.
 *
 * Until a layout is picked the scene's blocks are copied as written, as they
 * always were; with one, the built still is what goes to the generator.
 */
export function SceneFraming({ scene, onFraming }: Props) {
  const { t } = useTranslation()
  const view = useQuery(queries.sceneFrameView(scene.id))
  const data = view.data

  return (
    <section
      aria-label={t('scenes.framing.title')}
      className="flex flex-col gap-2.5 rounded-md border border-line bg-raise p-3"
    >
      <header className="flex flex-wrap items-center gap-2">
        <b className="text-sm font-semibold">{t('scenes.framing.title')}</b>
        <span className="text-2xs text-faint">{t('scenes.framing.hint')}</span>
        {scene.framing !== null && (
          <Button variant="link" className="ml-auto text-xs" onClick={() => onFraming(null)}>
            {t('scenes.framing.clear')}
          </Button>
        )}
      </header>
      {data?.block === null && (
        <p className="text-xs text-faint">{t('scenes.framing.noPictureBlock')}</p>
      )}
      <div className="grid gap-3 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
        <FramingPicker
          framing={scene.framing}
          layouts={data?.layouts ?? []}
          lettering={false}
          onChange={onFraming}
        />
        {data !== undefined && data.scheme !== null && data.still !== null && (
          <div className="group flex min-w-0 flex-col gap-2">
            <div className="flex h-36 items-center justify-center rounded-md bg-soft p-2">
              <SchemeView scheme={data.scheme} label={t('cover.schemeLabel')} fit />
            </div>
            {data.problems.map((problem, index) => (
              <p key={index} className="flex items-start gap-1.5 text-xs text-warn">
                <AlertTriangle aria-hidden className="mt-0.5 size-3.5 shrink-0" />
                {t(`cover.problem.${problem.kind}`, problem)}
              </p>
            ))}
            <header className="flex items-center gap-2">
              <span className="caption">{t('scenes.framing.still')}</span>
              <CopyButton
                value={data.still}
                label={t('scenes.framing.copy')}
                copiedLabel={t('cover.copied')}
                title={t('scenes.framing.copy')}
                className="ml-auto"
                onCopy={(ok) => {
                  if (!ok) say.failed(t('work.copyFailed'))
                }}
              />
            </header>
            <pre className="selectable rounded-md border border-line bg-soft px-2.5 py-2 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap text-dim">
              {data.still}
            </pre>
          </div>
        )}
      </div>
    </section>
  )
}
