import { useEffect, useMemo, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { BookPlus, Plus } from 'lucide-react'
import type { StyleBrick, StyleOrigin, StyleType } from '@/lib/api/types'
import { picturesAmong } from '@/lib/drop'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { useProfile, styleTypesOf, say as sayLabel } from '@/lib/useProfile'
import { styleIconOf } from '@/lib/styleIcon'
import { useDebounced } from '@/lib/useDebounced'
import { livingTypes } from '@/lib/styleBrick'
import { PICTURE, halfOfType, halvesOf } from '@/lib/phrases'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { EmptyState } from '@/components/ui/empty-state'
import { Skeleton, SkeletonGrid } from '@/components/ui/skeleton'
import { Frame, ListDetail, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { pictureIn, readPicture } from '@/features/styles/pastedPicture'
import { StyleBrickCard } from '@/features/styles/StyleBrickCard'
import { StyleDropCard } from '@/features/styles/StyleDropCard'
import { StyleEditor } from '@/features/styles/StyleEditor'
import { FromStylesDialog } from '@/features/styles/FromStylesDialog'
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

const ORIGINS = ['set', 'changed', 'own'] as const satisfies readonly StyleOrigin[]
const STATUSES = ['draft', 'ready', 'dropped'] as const

/** The narrowing the window does itself: the list is small, and read whole. */
interface Narrowing {
  family: string | undefined
  origin: StyleOrigin | undefined
  status: string | undefined
}

function narrowed(rows: StyleBrick[], { family, origin, status }: Narrowing): StyleBrick[] {
  return rows.filter(
    (brick) =>
      (family === undefined || brick.family === family) &&
      (origin === undefined || brick.origin === origin) &&
      (status === undefined || brick.status === status),
  )
}

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
  const allTypes = styleTypesOf(config)
  // The dictionary in its halves (v0.94): what a picture is built from, and
  // what each composition writes a text out of - the sound of a song.
  const halves = halvesOf(config, allTypes)
  const [halfKey, setHalfKey] = useState<string>(PICTURE)

  const [typeKey, setTypeKey] = useState<string | undefined>(undefined)
  const [family, setFamily] = useState<string | undefined>(undefined)
  const [origin, setOrigin] = useState<StyleOrigin | undefined>(undefined)
  const [status, setStatus] = useState<string | undefined>(undefined)
  const [text, setText] = useState('')
  // The list is a query; typing into it unthrottled would refetch per letter.
  const query = useDebounced(text, 200)
  // A style just made opens with its name ready to be typed over.
  const [fresh, setFresh] = useState<string | null>(null)

  const bricks = useQuery(queries.styleBricksMatching(typeKey ?? null, query))
  const house = useQuery(queries.houseStyles())
  const houseSet = new Set(house.data ?? [])
  const [fromTexts, setFromTexts] = useState(false)
  // Every style, whatever the filters: the open one is found here, so
  // narrowing the dictionary never closes the style being edited, and a new
  // style's name is kept apart from every other of its type.
  const everything = useQuery(queries.styleBricksMatching(null, ''))
  const counts = useQuery(queries.styleCounts())

  // The open style's half is the half shown: a link to a phrase opens the
  // sound, a link to an image style the picture.
  const openHalf =
    styleId === undefined
      ? undefined
      : halfOfType(halves, (everything.data ?? []).find((one) => one.id === styleId)?.type_key)
  const half = openHalf ?? halves.find((one) => one.key === halfKey) ?? halves[0]
  const types = half?.types ?? allTypes
  const inHalf = new Set(types.map((one) => one.key))
  // Only the picture's half is made of pictures dropped or pasted on it.
  const pastesPictures = half?.composition == null

  const countOf = useMemo(() => new Map(counts.data ?? []), [counts.data])
  const total = (counts.data ?? [])
    .filter(([key]) => inHalf.has(key))
    .reduce((sum, [, n]) => sum + n, 0)

  const open = (id: string | null) => {
    void navigate(id === null ? '/styles' : `/styles/${id}`)
  }

  // A new style is made in the type being looked at, unless that type is
  // retired: then in the first the craft still makes.
  const living = livingTypes(types)
  const current = types.find((one) => one.key === typeKey)
  const makeIn = current !== undefined && living.includes(current) ? current.key : living[0]?.key

  const make = useNewStyle({
    typeKey: makeIn ?? '',
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
    // A phrase is no picture: pasting one makes nothing in the sound.
    if (styleId !== undefined || types.length === 0 || making || pastesPictures === false) return
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
  }, [styleId, types.length, making, makeMutate, pastesPictures])

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

  const retired = current?.retired ?? null
  const families = current?.families ?? []

  const selected =
    styleId === undefined
      ? undefined
      : ((everything.data ?? []).find((one) => one.id === styleId) ??
        (bricks.data ?? []).find((one) => one.id === styleId))
  const filtered =
    typeKey !== undefined ||
    query !== '' ||
    family !== undefined ||
    origin !== undefined ||
    status !== undefined

  const dictionary = (
    <Loaded
      query={bricks}
      fill
      skeleton={<SkeletonGrid cells={6} columns={3} cellClassName="h-37.5" />}
    >
      {(all) => {
        const rows = narrowed(
          all.filter((brick) => inHalf.has(brick.type_key)),
          { family, origin, status },
        )
        return (
          // A hair of room around the grid, so a card's focus ring is not cut
          // by the edge of the scrolling box.
          <Scroll label={t('nav.styles')} contentClassName="flex flex-col gap-2.5 p-0.5">
            {/* The channel's own is not a style: said once, above the
                  dictionary, so nobody goes looking for the mark here. */}
            <p className="text-xs text-faint">{t('styles.brandNote')}</p>
            {current !== undefined && <TypeNote type={current} />}
            {rows.length === 0 &&
              retired === null &&
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
                    house={houseSet.has(brick.id)}
                    onOpen={() => {
                      setFresh(null)
                      open(brick.id)
                    }}
                  />
                )
              })}
              {/* A retired type makes nothing new: no dashed card under it -
                  nor does a phrase, which is no picture to drop. */}
              {retired === null && half?.composition === null && (
                <StyleDropCard onPaths={fromFiles} busy={making} />
              )}
            </ul>
          </Scroll>
        )
      }}
    </Loaded>
  )

  const detail =
    selected !== undefined ? (
      <StyleEditor
        key={selected.id}
        brick={selected}
        types={allTypes}
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
          {/* The halves, when the craft writes texts out of the dictionary
              too: a picture's parts and a song's sound are not one list. */}
          {halves.length > 1 && (
            <SegmentedControl
              aria-label={t('styles.half')}
              value={half?.key ?? PICTURE}
              onValueChange={(next) => {
                setHalfKey(next)
                setTypeKey(undefined)
                setFamily(undefined)
                if (styleId !== undefined) open(null)
              }}
            >
              {halves.map((one) => (
                <Segment key={one.key} value={one.key}>
                  {one.composition === null ? t('styles.picture') : sayLabel(one.composition.label)}
                </Segment>
              ))}
            </SegmentedControl>
          )}
          {/* The types as a row of chips, the way the storyboard narrows to a kind
              of shot: "show me every environment" is one click, and the chip that
              is on turns off - letting go of one leaves the group empty, which is
              every type. */}
          <ChipGroup
            aria-label={t('styles.type')}
            value={[chipOf(typeKey)]}
            onValueChange={(next) => {
              setTypeKey(typeOf(next[0]))
              // A family belongs to its type; another type starts unfiled.
              setFamily(undefined)
            }}
          >
            <Chip value={chipOf(undefined)} count={total}>
              {t('styles.allTypes')}
            </Chip>
            {types.map((one) => {
              const Icon = styleIconOf(one)
              const gone = one.retired !== undefined && one.retired !== null
              return (
                <Chip
                  key={one.key}
                  value={chipOf(one.key)}
                  count={countOf.get(one.key) ?? 0}
                  variant={gone ? 'dashed' : undefined}
                  title={gone ? sayLabel(one.retired) : undefined}
                >
                  <Icon aria-hidden className="size-3.5" />
                  {sayLabel(one.label)}
                </Chip>
              )
            })}
          </ChipGroup>
          {/* The second row: a type's families, then where a style came from
              and where it stands. Each group lets go the way the types do -
              pressing the chip that is on leaves the group empty, which is
              "any". */}
          <div className="flex w-full flex-wrap items-center gap-1.5">
            {families.length > 0 && (
              <ChipGroup
                aria-label={t('styles.family')}
                value={family === undefined ? [] : [family]}
                onValueChange={(next) => setFamily(next[0])}
              >
                {families.map((one) => (
                  <Chip key={one.key} value={one.key}>
                    {sayLabel(one.label)}
                  </Chip>
                ))}
              </ChipGroup>
            )}
            <ChipGroup
              aria-label={t('styles.originLabel')}
              value={origin === undefined ? [] : [origin]}
              onValueChange={(next) => setOrigin(next[0] as StyleOrigin | undefined)}
              className="ml-auto"
            >
              {ORIGINS.map((one) => (
                <Chip key={one} value={one}>
                  {t(`styles.originFilter.${one}`)}
                </Chip>
              ))}
            </ChipGroup>
            <ChipGroup
              aria-label={t('styles.statusLabel')}
              value={status === undefined ? [] : [status]}
              onValueChange={(next) => setStatus(next[0])}
            >
              {STATUSES.map((one) => (
                <Chip key={one} value={one}>
                  {t(`styles.statusFilter.${one}`)}
                </Chip>
              ))}
            </ChipGroup>
          </div>
          <Input
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={t('styles.search')}
            aria-label={t('styles.search')}
            className="max-w-60"
          />
          {/* The owner's own phrases, read out of their texts and offered
              to the dictionary (v0.94). */}
          {half?.composition != null && (
            <Button variant="soft" onClick={() => setFromTexts(true)} className="ml-auto">
              <BookPlus aria-hidden />
              {t('phrases.fromTexts')}
            </Button>
          )}
          <Button
            variant="primary"
            onClick={() => make.mutate(null)}
            disabled={making || makeIn === undefined}
            className="ml-auto"
          >
            <Plus aria-hidden />
            {t('styles.new')}
          </Button>
        </>
      }
    >
      {styleId === undefined ? dictionary : <ListDetail list={dictionary} detail={detail} />}
      {fromTexts && half?.composition != null && (
        <FromStylesDialog open onOpenChange={setFromTexts} composition={half.composition} />
      )}
    </Frame>
  )
}

/**
 * What the dictionary says above a type's bricks, when the craft has something
 * to say: a retired type's bricks are read but no longer made, and a type the
 * canon stands in for is not needed for a hero with a card.
 */
function TypeNote({ type }: { type: StyleType }) {
  const { t } = useTranslation()
  if (type.retired !== undefined && type.retired !== null) {
    return (
      <p className="rounded-lg border border-dashed border-line-2 bg-softer px-4 py-3 text-sm text-dim">
        <span className="font-semibold text-text">
          {t('styles.retiredTitle', { type: sayLabel(type.label) })}
        </span>{' '}
        {sayLabel(type.retired)} {t('styles.retiredBody')}
      </p>
    )
  }
  if (type.canon_kind !== undefined && type.canon_kind !== null) {
    return (
      <p className="rounded-lg border border-dashed border-line-2 bg-softer px-4 py-3 text-sm text-dim">
        {t('styles.canonStandsIn', { type: sayLabel(type.label) })}{' '}
        <Link to="/canon" className="text-accent underline-offset-2 hover:underline">
          {t('styles.openCanon')}
        </Link>
      </p>
    )
  }
  return null
}
