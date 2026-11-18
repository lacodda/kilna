import { useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Minus, Plus, Scan } from 'lucide-react'
import type { CardSummary } from '@/lib/api/types'
import { cardKindsOf } from '@/lib/canon'
import { CanonGlyph } from '@/lib/canonIcon'
import { queries } from '@/lib/query/queries'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Pane } from '@/components/ui/frame'
import { clipped, forceLayout, toneOf } from '@/features/canon/graph'

const WIDTH = 1000
const HEIGHT = 700
const ZOOMS = [0.5, 0.67, 0.8, 1, 1.25, 1.5, 2, 3] as const

/**
 * The whole canon as a map (v0.143, FR-18 18.4): every card a node in its
 * kind's glyph and colour, every relation a line with the first words of
 * what it says. The wheel and the buttons zoom, dragging the ground moves
 * the map, the chips hide kinds, and pressing a node opens its card.
 */
export function CanonGraph({
  cards,
  onOpen,
}: {
  cards: readonly CardSummary[]
  onOpen: (id: string) => void
}) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const links = useQuery(queries.canonLinks())
  const kinds = cardKindsOf(config)
  const present = kinds.filter((kind) => cards.some((card) => card.kind === kind))
  // The kinds hidden by the chips: every kind shows until one is let go,
  // whenever the cards arrive.
  const [hidden, setHidden] = useState<string[]>([])
  const [zoom, setZoom] = useState(3)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const dragging = useRef<{ x: number; y: number; from: { x: number; y: number } } | null>(null)

  const visible = cards.filter((card) => !hidden.includes(card.kind))
  const ids = visible.map((card) => card.id)
  const edges = (links.data ?? []).filter(
    (link) => ids.includes(link.from_id) && ids.includes(link.to_id),
  )
  const key = `${ids.join(',')}|${edges.map((edge) => edge.id).join(',')}`
  // Laid out again only when the cards or the relations change.
  const placed = useMemo(
    () =>
      forceLayout(
        ids,
        edges.map((edge) => ({ from: edge.from_id, to: edge.to_id })),
        { width: WIDTH, height: HEIGHT },
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key],
  )
  const scale = ZOOMS[zoom]!
  const zoomBy = (step: number) =>
    setZoom((at) => Math.min(ZOOMS.length - 1, Math.max(0, at + step)))
  const open = (id: string) => (event: KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      onOpen(id)
    }
  }
  const grab = (event: PointerEvent<SVGRectElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId)
    dragging.current = { x: event.clientX, y: event.clientY, from: pan }
  }
  const move = (event: PointerEvent<SVGRectElement>) => {
    const held = dragging.current
    if (held === null) return
    const box = event.currentTarget.ownerSVGElement?.getBoundingClientRect()
    const ratio = box === undefined || box.width === 0 ? 1 : WIDTH / box.width
    setPan({
      x: held.from.x + (event.clientX - held.x) * ratio,
      y: held.from.y + (event.clientY - held.y) * ratio,
    })
  }

  return (
    <Pane
      label={t('canon.graph')}
      bodyClassName="flex min-h-0 flex-col p-0"
      head={
        <div className="flex w-full flex-wrap items-center gap-2">
          <ChipGroup
            multiple
            aria-label={t('canon.graphKinds')}
            value={present.filter((kind) => !hidden.includes(kind))}
            onValueChange={(next) => setHidden(present.filter((kind) => !next.includes(kind)))}
          >
            {present.map((kind) => {
              const found = config.note_kinds?.find((one) => one.key === kind)
              return (
                <Chip key={kind} value={kind}>
                  <CanonGlyph kind={found} className="size-3.5" />
                  {found === undefined ? kind : sayLabel(found.label)}
                </Chip>
              )
            })}
          </ChipGroup>
          <span className="ml-auto flex items-center gap-1">
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('canon.zoomOut')}
              title={t('canon.zoomOut')}
              disabled={zoom === 0}
              onClick={() => zoomBy(-1)}
            >
              <Minus aria-hidden />
            </Button>
            <span className="w-10 text-center font-mono text-xs text-dim">
              {`${Math.round(scale * 100)}%`}
            </span>
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('canon.zoomIn')}
              title={t('canon.zoomIn')}
              disabled={zoom === ZOOMS.length - 1}
              onClick={() => zoomBy(1)}
            >
              <Plus aria-hidden />
            </Button>
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('canon.fitGraph')}
              title={t('canon.fitGraph')}
              onClick={() => {
                setZoom(3)
                setPan({ x: 0, y: 0 })
              }}
            >
              <Scan aria-hidden />
            </Button>
          </span>
        </div>
      }
    >
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        className="min-h-0 w-full flex-1 touch-none select-none"
        role="group"
        aria-label={t('canon.graphMap')}
        onWheel={(event) => zoomBy(event.deltaY < 0 ? 1 : -1)}
      >
        <rect
          width={WIDTH}
          height={HEIGHT}
          className="cursor-grab fill-transparent active:cursor-grabbing"
          onPointerDown={grab}
          onPointerMove={move}
          onPointerUp={() => {
            dragging.current = null
          }}
        />
        <g
          transform={`translate(${pan.x + (WIDTH / 2) * (1 - scale)} ${
            pan.y + (HEIGHT / 2) * (1 - scale)
          }) scale(${scale})`}
        >
          {edges.map((edge) => {
            const from = placed.get(edge.from_id)
            const to = placed.get(edge.to_id)
            if (from === undefined || to === undefined) return null
            return (
              <g key={edge.id}>
                <line
                  x1={from.x}
                  y1={from.y}
                  x2={to.x}
                  y2={to.y}
                  className="stroke-line-2"
                  strokeWidth={1}
                />
                {edge.label !== null && edge.label !== '' && (
                  <text
                    x={(from.x + to.x) / 2}
                    y={(from.y + to.y) / 2 - 3}
                    textAnchor="middle"
                    className="pointer-events-none fill-faint text-2xs"
                  >
                    {clipped(edge.label, 18)}
                  </text>
                )}
              </g>
            )
          })}
          {visible.map((card) => {
            const spot = placed.get(card.id)
            if (spot === undefined) return null
            const title = card.title ?? t('canon.untitled')
            return (
              <g
                key={card.id}
                role="link"
                tabIndex={0}
                aria-label={title}
                className="cursor-pointer outline-none focus-visible:[&>circle]:stroke-[3]"
                onClick={() => onOpen(card.id)}
                onKeyDown={open(card.id)}
              >
                <title>{title}</title>
                <circle
                  cx={spot.x}
                  cy={spot.y}
                  r={15}
                  strokeWidth={1.5}
                  className={toneOf(card.kind, kinds)}
                />
                <CanonGlyph
                  kind={config.note_kinds?.find((one) => one.key === card.kind)}
                  x={spot.x - 8}
                  y={spot.y - 8}
                  width={16}
                  height={16}
                  className="text-text"
                />
                <text
                  x={spot.x}
                  y={spot.y + 28}
                  textAnchor="middle"
                  className="pointer-events-none fill-dim text-xs"
                >
                  {clipped(title, 18)}
                </text>
              </g>
            )
          })}
        </g>
      </svg>
    </Pane>
  )
}
