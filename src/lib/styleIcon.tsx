import {
  Camera,
  Grid3x3,
  Layers,
  Move,
  Palette,
  Shapes,
  Shirt,
  TreePine,
  Type,
  User,
  type LucideIcon,
} from 'lucide-react'
import type { StyleType } from '@/lib/api'

/**
 * The glyphs a style type may carry, by the name the profile writes.
 *
 * A short, closed list, for the reason the actions and the marks have one: the
 * name lives in a JSON document the author edits by hand, and a list they can
 * read in the profile reference beats a thousand names they cannot. A name
 * outside it draws the generic shape rather than nothing — a type is never
 * left unmarked.
 *
 * The glyph matters more here than elsewhere: a picker of forty bricks under
 * nine types is read by shape first and by word second.
 */
export const STYLE_ICONS: Record<string, LucideIcon> = {
  palette: Palette,
  user: User,
  shirt: Shirt,
  tree: TreePine,
  type: Type,
  camera: Camera,
  move: Move,
  layers: Layers,
  grid: Grid3x3,
}

/** The names, for the reference and for a picker. */
export const STYLE_ICON_NAMES = Object.keys(STYLE_ICONS)

export function styleIconOf(type: Pick<StyleType, 'icon'> | undefined): LucideIcon {
  const name = type?.icon
  return (name !== undefined && name !== null ? STYLE_ICONS[name] : undefined) ?? Shapes
}
