import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown, Disc, ExternalLink, FolderMinus } from 'lucide-react'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { chipVariants } from '@/components/ui/chip'
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu'
import { CollectionMenuItems } from './CollectionMenuItems'
import { NewCollectionDialog } from './NewCollectionDialog'
import { useCollectionGestures } from './useCollectionGestures'

/**
 * The collection a work is in, in its header - and the way to change it
 * there: another album, none, or a new one made for it.
 *
 * The one chip of the header that does something, because the collection is
 * decided here as much as anywhere: writing the twelfth track is when it
 * goes on the album. Dashed while the work is in none, like every "not there
 * yet" of the line; absent in a craft with no kind of collection and none
 * made, where it would offer a choice of nothing.
 */
export function CollectionPicker({ work }: { work: Work }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { config } = useProfile()
  const collections = useQuery(queries.collections())
  const { add, arrange } = useCollectionGestures()
  const [making, setMaking] = useState(false)

  const all = collections.data ?? []
  if (all.length === 0 && config.collection_kinds.length === 0) return null
  const current = all.find((collection) => collection.id === work.collection_id)
  const busy = add.isPending || arrange.isPending

  return (
    <>
      <Menu>
        <MenuTrigger
          disabled={busy}
          title={t('collections.pickHint')}
          className={cn(
            chipVariants({ variant: current === undefined ? 'dashed' : 'outline' }),
            'target-min cursor-pointer gap-1 hover:border-line-2 hover:text-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent',
          )}
        >
          <Disc aria-hidden className="size-3" />
          <span className="max-w-48 truncate">
            {current === undefined ? t('collections.pickNone') : current.title}
          </span>
          <ChevronDown aria-hidden className="size-3" />
        </MenuTrigger>
        <MenuPopup align="start">
          <CollectionMenuItems
            collections={all}
            current={current?.id ?? null}
            onPick={(collection) => {
              if (collection.id === current?.id) return
              add.mutate({ collection, workIds: [work.id] })
            }}
            onNew={() => setMaking(true)}
          />
          {current !== undefined && (
            <>
              <MenuSeparator />
              <MenuItem onClick={() => void navigate(`/collections/${current.id}`)}>
                <ExternalLink aria-hidden className="size-3.5" />
                {t('collections.open', { title: current.title })}
              </MenuItem>
              <MenuItem
                onClick={() =>
                  arrange.mutate({
                    collection: current,
                    workIds: current.work_ids.filter((id) => id !== work.id),
                    message: t('collections.takenOut', { title: work.title }),
                  })
                }
              >
                <FolderMinus aria-hidden className="size-3.5" />
                {t('collections.takeOutOf', { title: current.title })}
              </MenuItem>
            </>
          )}
        </MenuPopup>
      </Menu>
      <NewCollectionDialog open={making} onOpenChange={setMaking} workIds={[work.id]} />
    </>
  )
}
