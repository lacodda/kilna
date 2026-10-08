import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { FolderPlus, Plus } from 'lucide-react'
import { today as todayOf } from '@/lib/month'
import { queries } from '@/lib/query/queries'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Skeleton, SkeletonGrid } from '@/components/ui/skeleton'
import { Frame, ListDetail, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { CollectionCard } from './CollectionCard'
import { CollectionDetail } from './CollectionDetail'
import { NewCollectionDialog } from './NewCollectionDialog'

/**
 * The albums, books and seasons: a grid of cards, each saying how full it is
 * against its goal and how good what it holds is, with a dashed card last
 * that makes a new one - the mockup's screen.
 *
 * The open collection is a panel beside the grid, the way an open style is:
 * the grid narrows to a column so the others stay in reach for switching,
 * and the collection gets the width its list of works needs. It is part of
 * the address, so back walks between collections and the catalogue's
 * "collection" chip can land on one.
 *
 * What goes into a collection mostly comes from the catalogue - ticked and
 * sent with "To collection", or carried onto one - which is where the works
 * are found. The page can add one at a time by name, for the track that is
 * missing.
 */
export function CollectionsView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { collectionId } = useParams()
  const [making, setMaking] = useState(false)

  const collections = useQuery(queries.collections())
  // The verdicts of what each holds come from the catalogue's rows, which
  // carry every work's speaking total. Not waited for: a card draws its goal
  // without them and fills the verdict in.
  const rows = useQuery(queries.catalogue())
  const today = todayOf()

  const open = (id: string | null) => {
    void navigate(id === null ? '/collections' : `/collections/${id}`)
  }

  const selected = (collections.data ?? []).find((one) => one.id === collectionId)

  const grid = (
    <Loaded
      query={collections}
      fill
      skeleton={<SkeletonGrid cells={4} columns={3} cellClassName="h-32" />}
    >
      {(all) => (
        // A hair of room around the grid, so a card's focus ring is not cut by
        // the edge of the scrolling box.
        <Scroll label={t('nav.collections')} contentClassName="flex flex-col gap-2.5 p-0.5">
          {all.length === 0 && (
            <EmptyState plain title={t('collections.none')} body={t('collections.noneBody')} />
          )}
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] items-start gap-2.5">
            {all.map((collection) => (
              <CollectionCard
                key={collection.id}
                collection={collection}
                rows={rows.data ?? []}
                today={today}
                open={collection.id === collectionId}
                onOpen={() => open(collection.id)}
              />
            ))}
            <li className="flex">
              {/* Dashed, like every "not there yet" of the line. */}
              <Button
                variant="ghost"
                size={null}
                onClick={() => setMaking(true)}
                className="min-h-32 w-full flex-col justify-center gap-1.5 rounded-lg border-dashed bg-softer p-4 text-center font-normal whitespace-normal"
              >
                <FolderPlus aria-hidden className="size-5 text-faint" />
                <span className="caption">{t('collections.new')}</span>
                <span className="text-sm text-faint">{t('collections.newHint')}</span>
              </Button>
            </li>
          </ul>
        </Scroll>
      )}
    </Loaded>
  )

  const detail =
    selected !== undefined ? (
      <CollectionDetail
        key={selected.id}
        collection={selected}
        rows={rows.data ?? []}
        today={today}
        onClose={() => open(null)}
      />
    ) : collections.isPending || collections.isFetching ? (
      // Waiting, not gone: a collection just made opens before the list that
      // holds it has come back.
      <Skeleton className="flex-1 rounded-lg" />
    ) : (
      <EmptyState
        title={t('collections.gone')}
        body={t('collections.goneBody')}
        className="flex-1"
      />
    )

  return (
    <Frame
      head={
        <Button variant="primary" className="ml-auto" onClick={() => setMaking(true)}>
          <Plus aria-hidden />
          {t('collections.new')}
        </Button>
      }
    >
      {collectionId === undefined ? grid : <ListDetail list={grid} detail={detail} />}
      <NewCollectionDialog
        open={making}
        onOpenChange={setMaking}
        onMade={(made) => open(made.id)}
      />
    </Frame>
  )
}
