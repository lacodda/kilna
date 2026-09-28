import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { picturesAmong } from '@/lib/drop'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { useProfile, styleTypesOf, say as sayLabel } from '@/lib/useProfile'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { EmptyState } from '@/components/ui/empty-state'
import { Skeleton, SkeletonGrid } from '@/components/ui/skeleton'
import { Frame, ListDetail, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { pictureIn, readPicture } from '@/features/styles/pastedPicture'
import { StyleBrickCard } from '@/features/styles/StyleBrickCard'
import { StyleDropCard } from '@/features/styles/StyleDropCard'
import { StyleEditor } from '@/features/styles/StyleEditor'
import { useNewStyle } from '@/features/styles/useNewStyle'

/*
 * What a type chip is called in its group. "All" is a chip of its own, and the
 * types carry a prefix, so a profile that names a type "all" cannot be taken
 * for it.
 */
const ALL_TYPES = 'all'
const chipOf = (type: string | undefined) => (type === undefined ? ALL_TYPES : `type:${type}`)
const typeOf = (chip: string | undefined) =>
  chip === undefined || chip === ALL_TYPES ? undefined : chip.slice('type:'.length)

/**
 * The workspace's style dictionary: the parts a picture prompt is built from.
 *
 * One screen for all of them, because the whole point of ADR 0031 is that
 * there is one dictionary and not several: a grid of cards, each led by its
 * pictures, with a dashed card last that makes a new style of whatever is
 * dropped on it. A craft that names no types has none, and the rail does not
 * offer this.
 *
 * **The open style is a panel of the screen, beside the dictionary** - the
 * way an open note is - rather than a dialog over it. The grid narrows to a
 * list's column while one is open, so the cards stay in reach for switching
 * and the style gets the width a paragraph of description and a row of
 * pictures need; a detail column beside a full grid would have left it a
 * third of the window. With none open the grid takes the whole body, as in
 * the mockup. The open style is part of the address, so back walks between
 * styles.
 */
export function StylesView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { styleId } = useParams()
  const { config } = useProfile()
  const types = styleTypesOf(config)

  const [typeKey, setTypeKey] = useState<string | undefined>(undefined)
  const [text, setText] = useState('')
  // The list is a query; typing into it unthrottled would refetch per letter.
  const query = useDebounced(text, 200)
  // A style just made opens with its name ready to be typed over.
  const [fresh, setFresh] = useState<string | null>(null)

  const bricks = useQuery(queries.styleBricksMatching(typeKey ?? null, query))
  // Every style, whatever the filters: the open one is found here, so
  // narrowing the dictionary never closes the style being edited, and a new
  // style's name is kept apart from every other of its type.
  const everything = useQuery(queries.styleBricksMatching(null, ''))
  const counts = useQuery(queries.styleCounts())

  const countOf = useMemo(() => new Map(counts.data ?? []), [counts.data])
  const total = useMemo(() => (counts.data ?? []).reduce((sum, [, n]) => sum + n, 0), [counts.data])

  const open = (id: string | null) => {
    void navigate(id === null ? '/styles' : `/styles/${id}`)
  }

  const make = useNewStyle({
    typeKey: typeKey ?? types[0]?.key ?? '',
    bricks: everything.data ?? [],
    onMade: (brick) => {
      // A search would hide what was just made from the dictionary it was
      // made in; the type filter cannot, the style is made in that type.
      setText('')
      setFresh(brick.id)
      open(brick.id)
    },
  })
  const makeMutate = make.mutate

  // One style at a time: a second gesture while the first is being made
  // would reach for the same "Untitled style", which its type already has.
  const making = make.isPending

  const fromFiles = (paths: string[]) => {
    if (making) return
    const { pictures, others } = picturesAmong(paths)
    if (others > 0) say.warn(t('styles.notPictures', { count: others }))
    if (pictures.length > 0) makeMutate({ paths: pictures })
  }

  // A picture pasted while no style is open makes a new one of it - the
  // dashed card's promise. With one open the paste is that style's
  // reference instead (see StyleReferences), so this listens only without.
  useEffect(() => {
    if (styleId !== undefined || types.length === 0 || making) return
    const onPaste = (event: ClipboardEvent) => {
      const file = pictureIn(event)
      if (file === undefined) return
      event.preventDefault()
      readPicture(file).then(
        (pasted) => makeMutate({ pasted }),
        (cause: unknown) => say.failed(cause),
      )
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [styleId, types.length, making, makeMutate])

  if (types.length === 0) {
    return (
      <Frame>
        <EmptyState
          title={t('styles.noDictionary')}
          body={t('styles.noDictionaryBody')}
          className="flex-1"
        />
      </Frame>
    )
  }

  const selected =
    styleId === undefined
      ? undefined
      : ((everything.data ?? []).find((one) => one.id === styleId) ??
        (bricks.data ?? []).find((one) => one.id === styleId))
  const filtered = typeKey !== undefined || query !== ''

  const dictionary = (
    <Loaded
      query={bricks}
      fill
      skeleton={<SkeletonGrid cells={6} columns={3} cellClassName="h-37.5" />}
    >
      {(rows) => (
        // A hair of room around the grid, so a card's focus ring is not cut
        // by the edge of the scrolling box.
        <Scroll label={t('nav.styles')} contentClassName="flex flex-col gap-2.5 p-0.5">
          {rows.length === 0 &&
            (filtered ? (
              // A search or a type that matched nothing: the way out is the
              // row above, and the dashed card still makes one of this type.
              <EmptyState plain variant="filtered" title={t('styles.noMatches')} />
            ) : (
              <EmptyState plain title={t('styles.empty')} body={t('styles.emptyBody')} />
            ))}
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] items-start gap-2.5">
            {rows.map((brick) => {
              const type = types.find((one) => one.key === brick.type_key)
              return (
                <StyleBrickCard
                  key={brick.id}
                  brick={brick}
                  type={type}
                  icon={styleIconOf(type)}
                  open={brick.id === styleId}
                  onOpen={() => {
                    setFresh(null)
                    open(brick.id)
                  }}
                />
              )
            })}
            <StyleDropCard onPaths={fromFiles} busy={making} />
          </ul>
        </Scroll>
      )}
    </Loaded>
  )

  const detail =
    selected !== undefined ? (
      <StyleEditor
        key={selected.id}
        brick={selected}
        types={types}
        bricks={everything.data ?? []}
        naming={fresh === selected.id}
        onClose={() => {
          setFresh(null)
          open(null)
        }}
      />
    ) : everything.isPending || everything.isFetching ? (
      // Waiting, not gone: a style just made opens before the list that
      // holds it has come back.
      <Skeleton className="flex-1 rounded-lg" />
    ) : (
      <EmptyState title={t('styles.gone')} body={t('styles.goneBody')} className="flex-1" />
    )

  return (
    <Frame
      head={
        <>
          {/* The types as a row of chips, the way the storyboard narrows to a kind
              of shot: "show me every environment" is one click, and the chip that
              is on turns off - letting go of one leaves the group empty, which is
              every type. */}
          <ChipGroup
            aria-label={t('styles.type')}
            value={[chipOf(typeKey)]}
            onValueChange={(next) => setTypeKey(typeOf(next[0]))}
          >
            <Chip value={chipOf(undefined)} count={total}>
              {t('styles.allTypes')}
            </Chip>
            {types.map((one) => {
              const Icon = styleIconOf(one)
              return (
                <Chip key={one.key} value={chipOf(one.key)} count={countOf.get(one.key) ?? 0}>
                  <Icon aria-hidden className="size-3.5" />
                  {sayLabel(one.label)}
                </Chip>
              )
            })}
          </ChipGroup>
          <Input
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={t('styles.search')}
            aria-label={t('styles.search')}
            className="max-w-60"
          />
          <Button
            variant="primary"
            onClick={() => make.mutate(null)}
            disabled={making}
            className="ml-auto"
          >
            <Plus aria-hidden />
            {t('styles.new')}
          </Button>
        </>
      }
    >
      {styleId === undefined ? dictionary : <ListDetail list={dictionary} detail={detail} />}
    </Frame>
  )
}
