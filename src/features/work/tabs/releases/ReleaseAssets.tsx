import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { Paperclip, Trash2 } from 'lucide-react'
import { attachAsset, detachAsset } from '@/lib/api/assets'
import type { Asset, ScheduledRelease } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { MediaPreview } from '@/components/MediaPreview'
import { Loaded } from '@/components/Loaded'

interface Props {
  release: ScheduledRelease
}

/**
 * The files that go out with one release: its thumbnail, its subtitles, the
 * cut that was uploaded.
 *
 * Kept on the release rather than on the work, because they belong to one
 * door - a clip's thumbnail is not its premiere's - and so they are not on
 * the work's Files tab. The backend has held them since the asset table did;
 * until v0.80 nothing in the window showed them or let one be attached.
 *
 * Any file, not only pictures: what a platform takes with a release is not
 * for kilna to narrow. Each is copied into the workspace, as every file is,
 * so a backup carries it.
 */
export function ReleaseAssets({ release }: Props) {
  const { t } = useTranslation()
  const [picking, setPicking] = useState(false)

  const files = useQuery(queries.releaseAssets(release.id))
  const refreshed = [keys.releaseAssets(release.id)]

  const attach = useAppMutation({
    mutationFn: (source: string) => attachAsset(source, { release_id: release.id }),
    failure: 'files.attachFailed',
    refresh: refreshed,
    onSuccess: (asset) => say.ok(t('files.attached', { name: asset.original_name ?? '' })),
  })

  const detach = useAppMutation({
    mutationFn: (asset: Asset) => detachAsset(asset.id),
    failure: 'files.detachFailed',
    refresh: refreshed,
    onSuccess: () => say.ok(t('files.detached')),
  })

  const pick = async () => {
    setPicking(true)
    try {
      const chosen = await open({ multiple: false })
      if (typeof chosen === 'string') attach.mutate(chosen)
    } catch (cause) {
      say.failedTo(t('files.attachFailed'), cause)
    } finally {
      setPicking(false)
    }
  }

  return (
    <section className="flex flex-col gap-2">
      <header className="flex items-center gap-2">
        <h4 className="caption">{t('releases.files.title')}</h4>
        {files.data !== undefined && files.data.length > 0 && (
          <span className="font-mono text-2xs text-faint tabular-nums">{files.data.length}</span>
        )}
        <Button
          size="sm"
          className="ml-auto"
          disabled={picking || attach.isPending}
          onClick={() => void pick()}
        >
          <Paperclip aria-hidden />
          {t('files.attach')}
        </Button>
      </header>

      <Loaded
        query={files}
        skeleton={<Skeleton className="h-16 w-full" />}
        isEmpty={(data) => data.length === 0}
        // Plain, and its way in is the button above it.
        emptyState={<p className="text-xs text-faint">{t('releases.files.none')}</p>}
        plain
      >
        {(data) => (
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(144px,1fr))] gap-2">
            {data.map((asset) => {
              const name = asset.original_name ?? asset.label ?? asset.id
              return (
                <li
                  key={asset.id}
                  className="flex flex-col overflow-hidden rounded-md border border-line bg-raise"
                >
                  <div className="flex h-20 items-center justify-center bg-soft">
                    <MediaPreview
                      path={asset.path}
                      alt={name}
                      className="max-h-full max-w-full object-contain"
                    />
                  </div>
                  <div className="flex min-w-0 items-center gap-1 px-2 py-1">
                    <span className="min-w-0 flex-1 truncate text-xs text-dim" title={name}>
                      {name}
                    </span>
                    <Button
                      variant="danger"
                      size="icon-xs"
                      title={t('files.detach')}
                      aria-label={t('files.detach')}
                      disabled={detach.isPending}
                      onClick={() => detach.mutate(asset)}
                    >
                      <Trash2 aria-hidden />
                    </Button>
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </Loaded>
    </section>
  )
}
