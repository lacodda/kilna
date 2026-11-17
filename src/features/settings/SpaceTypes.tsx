import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open, save } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { writeTextFile } from '@/lib/api/data'
import {
  addSpaceType,
  deleteSpaceType,
  readProfileDocument,
  renameSpaceType,
  spaceTypeDocument,
} from '@/lib/api/spaces'
import type { SpaceType } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { PromptDialog } from '@/components/AppDialog'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { Button } from '@/components/ui/button'
import { ListRow } from '@/components/ui/list-row'
import { PROFILE_FILES, profileFileName } from '@/features/settings/ProfileTools'
import { SpaceGlyph } from '@/features/spaces/look'

/**
 * The types this machine keeps (v0.112, ADR 0079): each a profile under a
 * name, offered when a space is made. Renamed, written out to a file, let go
 * - and one more read from a file. A type is made from a space's profile in
 * that space's settings.
 */
export function SpaceTypes() {
  const { t } = useTranslation()
  const types = useQuery(queries.spaceTypes())
  const [renaming, setRenaming] = useState<SpaceType | null>(null)
  const [busy, setBusy] = useState(false)

  const refresh = [keys.spaceTypes]
  const rename = useAppMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => renameSpaceType(id, name),
    refresh,
    failure: 'spaces.typeRenameFailed',
  })
  const remove = useAppMutation({
    mutationFn: deleteSpaceType,
    refresh,
    failure: 'spaces.typeDeleteFailed',
    onSuccess: () => say.ok(t('spaces.typeDeleted')),
  })
  const add = useAppMutation({
    mutationFn: addSpaceType,
    refresh,
    failure: 'spaces.typeImportFailed',
    onSuccess: (kept) => say.ok(t('spaces.typeImported', { name: kept.name })),
  })

  // A file dialog first, as the data section's gestures are: a dismissed
  // dialog is nothing happening.
  const run = (task: () => Promise<void>) => {
    setBusy(true)
    task()
      .catch((cause: unknown) => say.failed(cause))
      .finally(() => setBusy(false))
  }

  const doImport = () =>
    run(async () => {
      const source = await open({ title: t('editor.importTitle'), filters: PROFILE_FILES })
      if (typeof source !== 'string') return
      add.mutate(await readProfileDocument(source))
    })

  const doExport = (type: SpaceType) =>
    run(async () => {
      const destination = await save({
        defaultPath: profileFileName(type.name),
        title: t('editor.exportTitle'),
        filters: PROFILE_FILES,
      })
      if (destination === null) return
      const written = await writeTextFile(destination, await spaceTypeDocument(type.id))
      say.withAction(t('editor.profileExported', { path: written }), t('data.show'), () => {
        revealItemInDir(written).catch((cause: unknown) => say.failed(cause))
      })
    })

  const actionsOf = (type: SpaceType): RowAction[] => [
    { key: 'rename', label: t('spaces.rename'), onSelect: () => setRenaming(type) },
    { key: 'export', label: t('editor.exportProfile'), onSelect: () => doExport(type) },
    {
      key: 'delete',
      label: t('spaces.deleteType'),
      onSelect: () => remove.mutate(type.id),
      danger: true,
    },
  ]

  const listed = types.data ?? []

  return (
    <>
      <h3 className="caption">{t('spaces.types')}</h3>
      <p className="text-sm text-dim">{t('spaces.typesDescription')}</p>
      {listed.length > 0 && (
        <div className="flex flex-col">
          {listed.map((type) => (
            <ListRow
              key={type.id}
              start={<SpaceGlyph space={{ kind: type.kind, colour: null, mark: null }} />}
              description={type.description ?? t('spaces.typeKinds', { count: type.work_kinds })}
              end={
                <>
                  <span>{t(`spaces.kinds.${type.kind}`)}</span>
                  <RowMenu
                    actions={actionsOf(type)}
                    label={t('spaces.typeActions', { name: type.name })}
                  />
                </>
              }
            >
              {type.name}
            </ListRow>
          ))}
        </div>
      )}
      <div>
        <Button disabled={busy || add.isPending} onClick={doImport}>
          {t('spaces.importType')}
        </Button>
      </div>

      <PromptDialog
        open={renaming !== null}
        onOpenChange={(next) => {
          if (!next) setRenaming(null)
        }}
        title={t('spaces.renameTypeTitle')}
        label={t('spaces.typeName')}
        initialValue={renaming?.name ?? ''}
        confirmLabel={t('spaces.rename')}
        onSubmit={(name) => {
          if (renaming !== null) rename.mutate({ id: renaming.id, name })
          setRenaming(null)
        }}
      />
    </>
  )
}
