import { useTranslation } from 'react-i18next'
import { ChevronDown } from 'lucide-react'
import type { Collection } from '@/lib/api/types'
import { allOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { BulkActions } from '@/features/assistant/BulkActions'
import { CollectionMenuItems } from '@/features/collections/CollectionMenuItems'
import { ActionBar, ActionBarButton } from '@/components/ui/action-bar'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'

interface Props {
  /** The works ticked and on show: what every button here acts on. */
  workIds: readonly string[]
  /** A write is on its way; the bar waits for it rather than stacking a second. */
  busy: boolean
  /** The collections the works can be put in. */
  collections: readonly Collection[]
  onSetStatus: (status: string) => void
  onUnschedule: () => void
  onToCollection: (collection: Collection) => void
  onToNewCollection: () => void
  onDelete: () => void
  onClear: () => void
}

/**
 * What can be done to the ticked rows, while there are any.
 *
 * At the foot of the screen, under the table and centred, as the mockup has it
 * - it used to be inserted above the table, and every row moved down by its
 * height the moment the first box was ticked, so the second click landed on
 * the row below the one aimed at. Here the table keeps its top and gives up
 * the bar's height at the bottom, where nothing is being aimed at.
 *
 * Each action is its own button. They sat behind one "Actions" menu for a
 * while, on the argument that deleting a batch should not be one click from
 * the assistant; the mockup lays them out flat, and what guards a batch
 * delete is the trash and the undo on the toast, not a menu in the way of
 * every other action. It stands apart at the end, in the colour of something
 * you would rather not do by accident.
 *
 * A toolbar, so the whole bar is one stop on the Tab key and the arrows walk
 * its buttons.
 */
export function BulkBar({
  workIds,
  busy,
  collections,
  onSetStatus,
  onUnschedule,
  onToCollection,
  onToNewCollection,
  onDelete,
  onClear,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const statuses = allOf(profile.config, 'statuses')

  return (
    <ActionBar
      aria-label={t('catalogue.bulk.label')}
      className="mx-auto w-fit max-w-full flex-wrap gap-1.5 rounded-lg border border-line-2 bg-raise px-3 py-1.5 shadow-raise"
    >
      <span className="mr-1 text-sm font-semibold">
        {t('catalogue.chosen', { count: workIds.length })}
      </span>

      <Menu>
        <ActionBarButton
          disabled={busy}
          render={<MenuTrigger render={<Button size="sm" variant="ghost" />} />}
        >
          {t('catalogue.bulk.setStatus')}
          <ChevronDown aria-hidden />
        </ActionBarButton>
        <MenuPopup align="start" side="top">
          {statuses.map((status) => (
            <MenuItem key={status.key} onClick={() => onSetStatus(status.key)}>
              {sayLabel(status.label)}
            </MenuItem>
          ))}
        </MenuPopup>
      </Menu>

      <ActionBarButton
        disabled={busy}
        render={<Button size="sm" variant="ghost" />}
        onClick={onUnschedule}
      >
        {t('catalogue.bulk.unschedule')}
      </ActionBarButton>

      {/* The batch the collections screen waited for: tick the album's
          tracks here, where they are found, and send them in at once. */}
      <Menu>
        <ActionBarButton
          disabled={busy}
          render={<MenuTrigger render={<Button size="sm" variant="ghost" />} />}
        >
          {t('catalogue.bulk.toCollection')}
          <ChevronDown aria-hidden />
        </ActionBarButton>
        <MenuPopup align="start" side="top">
          <CollectionMenuItems
            collections={collections}
            onPick={onToCollection}
            onNew={onToNewCollection}
          />
        </MenuPopup>
      </Menu>

      <BulkActions workIds={workIds} onStarted={onClear} />

      <ActionBarButton
        disabled={busy}
        render={<Button size="sm" variant="danger" />}
        onClick={onDelete}
      >
        {t('catalogue.action.delete')}
      </ActionBarButton>

      <ActionBarButton render={<Button size="sm" variant="ghost" />} onClick={onClear}>
        {t('catalogue.clearSelection')}
      </ActionBarButton>
    </ActionBar>
  )
}
