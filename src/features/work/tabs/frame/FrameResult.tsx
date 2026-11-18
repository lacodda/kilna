import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-dialog'
import { ImagePlus, Trash2 } from 'lucide-react'
import type { Work } from '@/lib/api/types'
import { attachAsset, detachAsset } from '@/lib/api/assets'
import { CLIPS, PICTURES } from '@/lib/media'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Panel, SectionLabel } from '@/components/ui/panel'
import { MediaPreview } from '@/components/MediaPreview'

/** The kind a frame's result is kept under among the work's files. */
const FRAME = 'frame'

/**
 * What came back for a frame (v0.136, FR-07 7.3): the still a generator
 * drew, or the loop it made - a picture or an mp4 - as a strip in the
 * frame's dock, picked from disk and kept among the work's files.
 */
export function FrameResult({ work }: { work: Work }) {
  const { t } = useTranslation()
  const assets = useQuery(queries.assets(work.id))
  const results = (assets.data ?? []).filter((asset) => asset.kind === FRAME).reverse()
  const refresh = [keys.assetsFor(work.id), keys.cardCounts(work.id)]
  const attach = useAppMutation({
    mutationFn: (source: string) => attachAsset(source, { work_id: work.id, kind: FRAME }),
    refresh,
    onSuccess: () => say.ok(t('frame.result.added')),
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => detachAsset(id),
    refresh,
  })
  const pick = async () => {
    const chosen = await open({
      multiple: true,
      filters: [{ name: t('frame.result.files'), extensions: [...PICTURES, ...CLIPS] }],
    })
    for (const source of Array.isArray(chosen) ? chosen : chosen === null ? [] : [chosen]) {
      attach.mutate(source)
    }
  }

  return (
    <Panel role="region" aria-label={t('frame.result.title')} className="flex flex-col gap-2 p-2">
      <header className="flex flex-wrap items-center gap-2">
        <SectionLabel>{t('frame.result.title')}</SectionLabel>
        <Button
          size="xs"
          className="ml-auto"
          onClick={() => void pick()}
          disabled={attach.isPending}
        >
          <ImagePlus />
          {t('frame.result.pick')}
        </Button>
      </header>
      {results.length === 0 ? (
        <p className="text-2xs text-faint">{t('frame.result.none')}</p>
      ) : (
        <ul className="grid auto-cols-[6rem] grid-flow-col gap-2 overflow-x-auto p-1">
          {results.map((asset) => (
            <li key={asset.id} className="flex min-w-0 flex-col gap-1">
              <MediaPreview
                path={asset.path}
                alt={asset.original_name ?? ''}
                className="aspect-video w-full rounded-md object-cover"
              />
              <span className="flex items-center gap-1">
                <span className="min-w-0 flex-1 truncate text-2xs text-faint">
                  {asset.original_name ?? ''}
                </span>
                <Button
                  variant="icon"
                  size="icon-xs"
                  aria-label={t('frame.result.remove')}
                  disabled={remove.isPending}
                  onClick={() => remove.mutate(asset.id)}
                >
                  <Trash2 aria-hidden />
                </Button>
              </span>
            </li>
          ))}
        </ul>
      )}
    </Panel>
  )
}
