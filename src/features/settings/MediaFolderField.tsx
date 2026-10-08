import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { setMediaRoot } from '@/lib/api/folders'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Skeleton } from '@/components/ui/skeleton'

/**
 * The media folder (v0.93, ADR 0057): where this workspace keeps its media on
 * this machine, and under it the folder each kind names for its works.
 *
 * Here rather than in the profile's editor because it is not the profile's:
 * a place on one machine, kept out of the log a sync would carry and out of
 * every export, the way which profile is open is. What *is* the profile's - how a kind names its
 * folder - is listed under it as a reminder of where each work will look,
 * and edited where the rest of the kind is.
 */
export function MediaFolderField() {
  const { t } = useTranslation()
  const { config } = useProfile()
  const root = useQuery(queries.mediaRoot())

  const keep = useAppMutation({
    mutationFn: (path: string | null) => setMediaRoot(path),
    failure: 'data.mediaFolderFailed',
    // Every work's folder is read against the root: all of them at once.
    refresh: [keys.mediaRoot, keys.folders],
    onSuccess: (now) => {
      say.ok(
        now === null ? t('data.mediaFolderForgotten') : t('data.mediaFolderSet', { path: now }),
      )
    },
  })

  const choose = async () => {
    try {
      const chosen = await open({
        directory: true,
        title: t('data.mediaFolderTitle'),
        defaultPath: root.data ?? undefined,
      })
      if (typeof chosen === 'string') keep.mutate(chosen)
    } catch (cause) {
      say.failed(cause)
    }
  }

  const named = config.work_kinds.filter((kind) => (kind.folder ?? null) !== null)

  return (
    <FieldGroup label={t('data.mediaFolder')} help={t('data.mediaFolderHint')}>
      {root.isPending ? (
        <Skeleton className="h-3 w-72" />
      ) : root.data == null ? (
        <p className="text-sm text-dim">{t('data.mediaFolderNone')}</p>
      ) : (
        <code className="selectable font-mono text-xs break-all text-text">{root.data}</code>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          className="self-start"
          disabled={keep.isPending}
          onClick={() => {
            void choose()
          }}
        >
          {root.data == null ? t('data.mediaFolderChoose') : t('data.mediaFolderChange')}
        </Button>
        {root.data != null && (
          <Button
            variant="ghost"
            className="self-start"
            disabled={keep.isPending}
            onClick={() => keep.mutate(null)}
          >
            {t('data.mediaFolderForget')}
          </Button>
        )}
      </div>
      {named.length > 0 && (
        <div className="flex flex-col gap-1">
          <p className="text-xs text-dim">{t('data.mediaFolderKinds')}</p>
          <dl className="grid grid-cols-[minmax(0,9rem)_minmax(0,1fr)] gap-x-3 gap-y-0.5 text-xs">
            {config.work_kinds.map((kind) => (
              <div key={kind.key} className="contents">
                <dt className="truncate text-dim">{sayLabel(kind.label)}</dt>
                <dd className="truncate font-mono text-text">
                  {kind.folder ?? (
                    <span className="font-sans text-faint">{t('data.mediaFolderNoKind')}</span>
                  )}
                </dd>
              </div>
            ))}
          </dl>
        </div>
      )}
    </FieldGroup>
  )
}
