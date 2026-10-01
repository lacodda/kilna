import type { ReactNode } from 'react'
import type { Scheme, SchemePaint, SchemeRect } from '@/lib/api/types'
import { cn } from '@/lib/utils'

interface Props {
  scheme: Scheme
  /** What the drawing says, for whoever cannot see it. */
  label: string
  /** Drawn inside a control that already says what it is - a layout's
      button - and so hidden from a screen reader, which would otherwise
      read the name twice. */
  decorative?: boolean
  className?: string
  /** Drawn over the scheme in the box the mark goes in: the mark's own file
      when it is laid over the picture at export. */
  mark?: ReactNode
}

/**
 * A picture's built frame, drawn: where the hero stands and how big, where
 * the title goes, the disc of the accent, the box the mark is laid in.
 *
 * Every shape is computed on the Rust side from the same settings the
 * prompt's sentences are written from (ADR 0049) - this draws what it is
 * sent and decides nothing, so the scheme cannot show a picture the prompt
 * does not ask for. The colours are the picture's own, not the window's
 * theme: a scheme on a paper ground is light in a dark window too.
 */
export function SchemeView({ scheme, label, className, mark, decorative }: Props) {
  const fill = (paint: SchemePaint) => scheme.colours[paint === 'accent' ? 'accent' : paint]
  return (
    // The box takes the picture's shape, so an overlay placed in shares of it
    // lands where the scheme says.
    <span
      className={cn('relative block', className)}
      style={{ aspectRatio: `${scheme.width} / ${scheme.height}` }}
    >
      <svg
        viewBox={`0 0 ${scheme.width} ${scheme.height}`}
        role={decorative === true ? undefined : 'img'}
        aria-label={decorative === true ? undefined : label}
        aria-hidden={decorative === true ? true : undefined}
        preserveAspectRatio="xMidYMid meet"
        className="block size-full overflow-hidden rounded-sm"
      >
        {scheme.shapes.map((shape, index) => {
          const paint = fill(shape.paint)
          // The accent's disc sits behind the hero like a print's moon:
          // softened, so the figure in front of it still reads.
          const opacity = shape.paint === 'accent' ? 0.85 : undefined
          switch (shape.shape) {
            case 'rect':
              return (
                <rect
                  key={index}
                  x={shape.rect.x}
                  y={shape.rect.y}
                  width={shape.rect.w}
                  height={shape.rect.h}
                  fill={paint}
                  opacity={opacity}
                />
              )
            case 'circle':
              return (
                <circle
                  key={index}
                  cx={shape.cx}
                  cy={shape.cy}
                  r={shape.r}
                  fill={paint}
                  opacity={opacity}
                />
              )
            case 'ellipse':
              return (
                <ellipse
                  key={index}
                  cx={shape.cx}
                  cy={shape.cy}
                  rx={shape.rx}
                  ry={shape.ry}
                  fill={paint}
                  opacity={opacity}
                />
              )
            case 'path':
              return <path key={index} d={shape.d} fill={paint} opacity={opacity} />
          }
        })}
        <rect
          x={0.5}
          y={0.5}
          width={scheme.width - 1}
          height={scheme.height - 1}
          fill="none"
          stroke={scheme.colours.ink}
          strokeOpacity={0.25}
        />
      </svg>
      {mark !== undefined && scheme.mark !== null && (
        <span className="pointer-events-none absolute" style={boxStyle(scheme.mark, scheme)}>
          {mark}
        </span>
      )}
    </span>
  )
}

/** A rectangle of the scheme as a share of the drawing, for an overlay. */
function boxStyle(rect: SchemeRect, scheme: Scheme) {
  return {
    left: `${(rect.x / scheme.width) * 100}%`,
    top: `${(rect.y / scheme.height) * 100}%`,
    width: `${(rect.w / scheme.width) * 100}%`,
    height: `${(rect.h / scheme.height) * 100}%`,
  }
}
