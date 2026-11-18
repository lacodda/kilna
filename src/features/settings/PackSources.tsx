import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { open } from '@tauri-apps/plugin-dialog'
import type { PackSource } from '@/lib/api/types'
import { addStyleSource, removeStyleSource, setPacksOffline } from '@/lib/api/packs'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { PromptDialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'

/** How a source is said: the snapshot by name, a folder or a site by itself. */
function sourceWord(source: PackSource, builtin: string): string {
  return source.kind === 'builtin' ? builtin : source.location
}

/**
 * Where style packs are read from (v0.131, FR-06): the snapshot shipped with
 * kilna, always; folders and sites the person adds; and whether to read the
 * sites at all. The machine's, whatever space is open.
 */
export function PackSources() {
  const { t } = useTranslation()
  const catalogue = useQuery(queries.stylePacks())
  const [asking, setAsking] = useState(false)
  const refresh = [keys.stylePacks]

  const add = useAppMutation({
    mutationFn: addStyleSource,
    refresh,
    failure: 'packs.sourceFailed',
  })
  const remove = useAppMutation({
    mutationFn: removeStyleSource,
    refresh,
    failure: 'packs.sourceFailed',
  })
  const offline = useAppMutation({
    mutationFn: setPacksOffline,
    refresh,
    failure: 'packs.sourceFailed',
  })

  const sources = catalogue.data?.sources ?? [{ kind: 'builtin' } as PackSource]
  return (
    <div className="flex flex-col gap-2">
      <h3 className="caption">{t('packs.sources')}</h3>
      <p className="text-xs text-dim">{t('packs.sourcesHint')}</p>
      <ul className="flex flex-col gap-1 text-sm">
        {sources.map((source) => (
          <li key={sourceWord(source, 'builtin')} className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate">
              {sourceWord(source, t('packs.builtin'))}
            </span>
            {source.kind !== 'builtin' && (
              <Button size="sm" variant="danger" onClick={() => remove.mutate(source)}>
                {t('packs.removeSource')}
              </Button>
            )}
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          onClick={() => {
            open({ directory: true, title: t('packs.addFolder') })
              .then((picked) => {
                if (typeof picked === 'string') add.mutate({ kind: 'folder', location: picked })
              })
              .catch((cause: unknown) => say.failed(cause))
          }}
        >
          {t('packs.addFolder')}
        </Button>
        <Button size="sm" onClick={() => setAsking(true)}>
          {t('packs.addUrl')}
        </Button>
      </div>
      <Switch
        checked={catalogue.data?.offline ?? false}
        onCheckedChange={(next) => offline.mutate(next)}
      >
        {t('packs.offline')}
      </Switch>
      <PromptDialog
        open={asking}
        onOpenChange={setAsking}
        title={t('packs.addUrl')}
        description={t('packs.addUrlHint')}
        label={t('packs.url')}
        confirmLabel={t('packs.add')}
        onSubmit={(url) => {
          setAsking(false)
          add.mutate({ kind: 'url', location: url.trim() })
        }}
      />
    </div>
  )
}
