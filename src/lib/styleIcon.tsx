import {
  AudioWaveform,
  Camera,
  Cloud,
  Disc3,
  Drum,
  Gauge,
  Guitar,
  MicVocal,
  SlidersHorizontal,
  TrendingUp,
  WavesHorizontal,
  Droplet,
  Grid3x3,
  Layers,
  Move,
  Palette,
  Shapes,
  Shirt,
  Square,
  Tag,
  TreePine,
  Type,
  User,
  type LucideIcon,
  type LucideProps,
} from 'lucide-react'
import { createElement } from 'react'
import type { StyleType } from '@/lib/api/types'

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
const STYLE_ICONS: Record<string, LucideIcon> = {
  palette: Palette,
  user: User,
  shirt: Shirt,
  tree: TreePine,
  type: Type,
  camera: Camera,
  move: Move,
  layers: Layers,
  grid: Grid3x3,
  tag: Tag,
  square: Square,
  droplet: Droplet,
  // The types of sound (v0.94).
  disc: Disc3,
  mic: MicVocal,
  drum: Drum,
  guitar: Guitar,
  waves: WavesHorizontal,
  cloud: Cloud,
  waveform: AudioWaveform,
  gauge: Gauge,
  trending: TrendingUp,
  sliders: SlidersHorizontal,
}

export function styleIconOf(type: Pick<StyleType, 'icon'> | undefined): LucideIcon {
  const name = type?.icon
  return (name !== undefined && name !== null ? STYLE_ICONS[name] : undefined) ?? Shapes
}

/** A type's glyph, drawn: for a component that shows one type rather than a
 *  list of them, where the glyph is looked up once per render. */
export function StyleIcon({
  of,
  ...props
}: { of: Pick<StyleType, 'icon'> | undefined } & LucideProps) {
  return createElement(styleIconOf(of), props)
}
