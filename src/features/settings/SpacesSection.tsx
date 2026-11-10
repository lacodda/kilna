import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { Plus } from 'lucide-react'
import { forgetSpace, renameSpace, restoreSpace, restyleSpace, trashSpace } from '@/lib/api/spaces'
import type { Space } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Dialog, PromptDialog } from '@/components/AppDialog'
import { RowMenu, type RowAction } from '@/components/RowMenu'
import { Button } from '@/components/ui/button'
import { ListRow } from '@/components/ui/list-row'
import { LookPicker } from '@/features/spaces/LookPicker'
import { NewSpaceDialog } from '@/features/spaces/NewSpaceDialog'
import { SpaceGlyph, kindLook } from '@/features/spaces/look'
import { useOpenSpace } from '@/features/spaces/useOpenSpace'

/**
 * The spaces this machine knows (ADR 0077): each a folder with a database of
 * its own. Renamed, given another look, opened, put in the trash and taken
 * back - and forgotten from there, which takes it off this list and leaves its
 * folder where it is: kilna never deletes a space's files.
 */
export function SpacesSection() {
  const { t } = useTranslation()
  const spaces = useQuery(queries.spaces())
  const current = useQuery(queries.currentSpace())
  const open = useOpenSpace()
  const [making, setMaking] = useState(false)
  const [renaming, setRenaming] = useState<Space | null>(null)
  const [restyling, setRestyling] = useState<Space | null>(null)

  const refresh = [keys.spaces]
  const rename = useAppMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) => renameSpace(id, name),
    refresh,
    failure: 'spaces.renameFailed',
  })
  const trash = useAppMutation({ mutationFn: trashSpace, refresh, failure: 'spaces.trashFailed' })
  const restore = useAppMutation({
    mutationFn: restoreSpace,
    refresh,
    failure: 'spaces.restoreFailed',
  })
  const forget = useAppMutation({
    mutationFn: forgetSpace,
    refresh,
    failure: 'spaces.forgetFailed',
    onSuccess: () => say.ok(t('spaces.forgotten')),
  })

  const reveal = (path: string) => {
    revealItemInDir(path).catch((cause: unknown) => say.failed(cause))
  }

  const actionsOf = (space: Space): RowAction[] => {
    const actions: RowAction[] = []
    if (!space.open && space.reachable && space.trashed_at === null) {
      actions.push({ key: 'open', label: t('spaces.open'), onSelect: () => open.mutate(space.id) })
    }
    if (space.reachable) {
      actions.push(
        { key: 'rename', label: t('spaces.rename'), onSelect: () => setRenaming(space) },
        { key: 'look', label: t('spaces.look'), onSelect: () => setRestyling(space) },
        { key: 'reveal', label: t('spaces.reveal'), onSelect: () => reveal(space.path) },
      )
    }
    if (space.trashed_at === null) {
      if (!space.open) {
        actions.push({
          key: 'trash',
          label: t('spaces.trash'),
          onSelect: () => trash.mutate(space.id),
          danger: true,
        })
      }
    } else {
      actions.push(
        { key: 'restore', label: t('spaces.restore'), onSelect: () => restore.mutate(space.id) },
        {
          key: 'forget',
          label: t('spaces.forget'),
          onSelect: () => forget.mutate(space.id),
          danger: true,
        },
      )
    }
    return actions
  }

  const listed = spaces.data ?? []
  const live = listed.filter((space) => space.trashed_at === null)
  const trashed = listed.filter((space) => space.trashed_at !== null)

  const row = (space: Space) => (
    <ListRow
      key={space.id}
      start={<SpaceGlyph space={space} />}
      description={space.path}
      end={
        <>
          <span>
            {space.open
              ? t('spaces.openNow')
              : !space.reachable
                ? t('spaces.unreachable')
                : space.kind !== null
                  ? t(`spaces.kinds.${space.kind}`)
                  : ''}
          </span>
          <RowMenu actions={actionsOf(space)} label={t('spaces.actions', { name: space.name })} />
        </>
      }
    >
      {space.name}
    </ListRow>
  )

  return (
    <section className="flex max-w-160 flex-col gap-4">
      <p className="text-sm text-dim">{t('spaces.sectionDescription')}</p>
      {current.data?.portable === true && (
        <p className="text-sm text-dim">{t('spaces.portable')}</p>
      )}
      <div>
        <Button onClick={() => setMaking(true)}>
          <Plus aria-hidden />
          {t('spaces.new')}
        </Button>
      </div>
      <div className="flex flex-col">{live.map(row)}</div>
      {trashed.length > 0 && (
        <>
          <h3 className="caption">{t('spaces.trashed')}</h3>
          <p className="text-sm text-dim">{t('spaces.forgetDescription')}</p>
          <div className="flex flex-col">{trashed.map(row)}</div>
        </>
      )}

      <NewSpaceDialog open={making} onOpenChange={setMaking} />
      <PromptDialog
        open={renaming !== null}
        onOpenChange={(next) => {
          if (!next) setRenaming(null)
        }}
        title={t('spaces.renameTitle')}
        label={t('spaces.name')}
        initialValue={renaming?.name ?? ''}
        confirmLabel={t('spaces.rename')}
        onSubmit={(name) => {
          if (renaming !== null) rename.mutate({ id: renaming.id, name })
          setRenaming(null)
        }}
      />
      {restyling !== null && <RestyleDialog space={restyling} onClose={() => setRestyling(null)} />}
    </section>
  )
}

/** Another colour and mark for one space. */
function RestyleDialog({ space, onClose }: { space: Space; onClose: () => void }) {
  const { t } = useTranslation()
  const fallback = kindLook(space.kind)
  const [colour, setColour] = useState(space.colour ?? fallback.colour)
  const [mark, setMark] = useState(space.mark ?? fallback.mark)
  const save = useAppMutation({
    mutationFn: () => restyleSpace(space.id, { colour, mark }),
    refresh: [keys.spaces],
    failure: 'spaces.restyleFailed',
    onSuccess: onClose,
  })

  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
      title={t('spaces.lookTitle', { name: space.name })}
      footer={
        <Button variant="primary" disabled={save.isPending} onClick={() => save.mutate()}>
          {t('spaces.save')}
        </Button>
      }
    >
      <LookPicker
        colour={colour}
        mark={mark}
        onChange={(next) => {
          if (next.colour !== undefined) setColour(next.colour)
          if (next.mark !== undefined) setMark(next.mark)
        }}
      />
    </Dialog>
  )
}
