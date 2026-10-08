import { useTranslation } from 'react-i18next'
import { Check, FolderPlus } from 'lucide-react'
import type { Collection } from '@/lib/api/types'
import {
  MenuCheckboxIndicator,
  MenuCheckboxItem,
  MenuGroup,
  MenuGroupLabel,
  MenuItem,
  MenuSeparator,
} from '@/components/ui/menu'

interface Props {
  collections: readonly Collection[]
  /** The collection the works are in now, ticked; none for a batch. */
  current?: string | null
  onPick: (collection: Collection) => void
  /** Make a new one for them - the last entry, always there. */
  onNew: () => void
}

/**
 * The collections to put works in, as the entries of a menu: one per
 * collection with how many it holds, then "New collection".
 *
 * Shared by the catalogue's bar and a work's header, so the two doors list
 * the same albums the same way. "New collection" stands even when there is
 * none to list: the first album is made from where its works are.
 */
export function CollectionMenuItems({ collections, current = null, onPick, onNew }: Props) {
  const { t } = useTranslation()
  return (
    <>
      {collections.length > 0 && (
        <MenuGroup>
          <MenuGroupLabel>{t('collections.putIn')}</MenuGroupLabel>
          {collections.map((collection) => (
            <MenuCheckboxItem
              key={collection.id}
              checked={collection.id === current}
              closeOnClick
              onCheckedChange={() => onPick(collection)}
            >
              <MenuCheckboxIndicator>
                <Check className="size-3.5" aria-hidden />
              </MenuCheckboxIndicator>
              <span className="min-w-0 flex-1 truncate">{collection.title}</span>
              <span className="font-mono text-xs text-faint tabular-nums">
                {collection.work_ids.length}
              </span>
            </MenuCheckboxItem>
          ))}
        </MenuGroup>
      )}
      {collections.length > 0 && <MenuSeparator />}
      <MenuItem onClick={onNew}>
        <FolderPlus aria-hidden className="size-3.5" />
        {t('collections.newEllipsis')}
      </MenuItem>
    </>
  )
}
