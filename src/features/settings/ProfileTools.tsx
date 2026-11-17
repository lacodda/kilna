import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { open, save } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { writeTextFile } from '@/lib/api/data'
import { profileDocument, readProfileDocument, saveSpaceType } from '@/lib/api/spaces'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { PromptDialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { useProfileDraft } from '@/features/settings/useProfileDraft'

/** The name a profile's file is offered under: `night-shift.kilna-profile.json`. */
export function profileFileName(name: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '')
  return `${slug === '' ? 'profile' : slug}.kilna-profile.json`
}

/** What the open dialog offers when a profile's file is picked. */
export const PROFILE_FILES = [{ name: 'kilna profile', extensions: ['json'] }]

/**
 * The profile as a whole thing (v0.112, ADR 0079): written out to a file,
 * read from one, and kept as a type new spaces are made of.
 *
 * What goes out - to a file or into a type - is the profile as it is saved,
 * not the draft on the screen: a document holding edits nobody saved would be
 * a third version of the profile, in neither the space nor the editor. And
 * what comes in lands as the draft: looked over, then saved by the bar at the
 * foot or thrown away, as any other edit is.
 */
export function ProfileTools({ dirty }: { dirty: boolean }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const { edit } = useProfileDraft()
  const [busy, setBusy] = useState(false)
  const [naming, setNaming] = useState(false)

  const keep = useAppMutation({
    mutationFn: saveSpaceType,
    refresh: [keys.spaceTypes],
    failure: 'spaces.typeSaveFailed',
    onSuccess: (kept) => say.ok(t('spaces.typeSaved', { name: kept.name })),
  })

  // Each opens an OS dialog first, so they are not mutations in the query
  // sense; a dismissed dialog is nothing happening, and nothing is said.
  const run = (task: () => Promise<void>) => {
    setBusy(true)
    task()
      .catch((cause: unknown) => say.failed(cause))
      .finally(() => setBusy(false))
  }

  const doExport = () =>
    run(async () => {
      const destination = await save({
        defaultPath: profileFileName(profile.name),
        title: t('editor.exportTitle'),
        filters: PROFILE_FILES,
      })
      if (destination === null) return
      const written = await writeTextFile(destination, await profileDocument())
      say.withAction(t('editor.profileExported', { path: written }), t('data.show'), () => {
        revealItemInDir(written).catch((cause: unknown) => say.failed(cause))
      })
    })

  const doImport = () =>
    run(async () => {
      const source = await open({ title: t('editor.importTitle'), filters: PROFILE_FILES })
      if (typeof source !== 'string') return
      const document = await readProfileDocument(source)
      edit(() => document.config)
      say.ok(t('editor.profileImported', { name: document.name }))
    })

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" disabled={busy} onClick={doExport}>
          {t('editor.exportProfile')}
        </Button>
        <Button size="sm" disabled={busy} onClick={doImport}>
          {t('editor.importProfile')}
        </Button>
        <Button size="sm" disabled={busy || keep.isPending} onClick={() => setNaming(true)}>
          {t('editor.saveAsType')}
        </Button>
      </div>
      <p className="text-xs text-dim">{t(dirty ? 'editor.toolsHintDirty' : 'editor.toolsHint')}</p>

      <PromptDialog
        open={naming}
        onOpenChange={setNaming}
        title={t('spaces.saveTypeTitle')}
        description={t('spaces.saveTypeDescription')}
        label={t('spaces.typeName')}
        initialValue={profile.name}
        confirmLabel={t('spaces.saveTypeConfirm')}
        onSubmit={(name) => {
          keep.mutate(name)
          setNaming(false)
        }}
      />
    </div>
  )
}
