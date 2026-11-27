import { useRef, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Plus, Shuffle, Trash2, Upload } from 'lucide-react'
import type {
  BlotShape,
  GroundPattern,
  ShortBlot,
  ShortStyle,
  ShortVariant,
  WaveKind,
} from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { artSrc } from '@/lib/shorts/ink'
import { freeKey, worn } from '@/lib/shorts/style'
import { say } from '@/lib/toast'
import { Select } from '@/components/AppSelect'
import { ColourField } from '@/components/ColourField'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'
import { Section } from '@/features/cover/Section'
import { useProfileDraft } from '@/features/settings/useProfileDraft'
import { ShortFrame } from '@/features/shorts/ShortFrame'

const PATTERNS: readonly GroundPattern[] = ['blots', 'dots', 'grid', 'stripes', 'plain']
const SHAPES: readonly BlotShape[] = ['splat', 'drip', 'spray', 'smear']
const WAVES: readonly WaveKind[] = ['solid', 'lines', 'bars']

/** The largest picture kept in the style as text: a sign is a mark, not a
 *  photograph, and the style travels with the profile. */
const ART_LIMIT = 400_000

/**
 * The constructor of the space's short style (v0.168): the blocks a frame is
 * built of, each with its settings and its switch, the variants the style is
 * worn in, and one frame drawn live beside them.
 *
 * What is edited is the profile's draft, saved from the bar at the foot of
 * the pane like the rest of the profile - so a style half-built is not what
 * the next short is built in.
 */
export function ShortsSection() {
  const { t } = useTranslation()
  const { config, edit } = useProfileDraft()
  const fallback = useQuery(queries.shortStyleDefault()).data
  // The variant the frame is drawn in; none draws the style as it is.
  const [shown, setShown] = useState<string | null>(null)

  const style = config.shorts ?? fallback
  if (style === undefined) return null

  const change = (patch: Partial<ShortStyle>) => {
    edit((current) => ({ ...current, shorts: { ...(current.shorts ?? style), ...patch } }))
  }
  const variant = style.variants.find((entry) => entry.key === shown)
  const setVariant = (key: string, patch: Partial<ShortVariant>) => {
    change({
      variants: style.variants.map((entry) => (entry.key === key ? { ...entry, ...patch } : entry)),
    })
  }

  const on = (checked: boolean, flip: (next: boolean) => void) => (
    <Checkbox checked={checked} onCheckedChange={flip}>
      {t('shorts.on')}
    </Checkbox>
  )

  return (
    <div className="flex flex-wrap items-start gap-6">
      <div className="flex min-w-80 flex-1 flex-col">
        <p className="max-w-2xl pb-2 text-xs text-dim">{t('shorts.lead')}</p>

        <Section id="shorts.colours" title={t('shorts.colours')} hint={t('shorts.coloursHint')}>
          <Row label={t('shorts.accent')}>
            <ColourField
              label={t('shorts.accent')}
              value={style.accent}
              onChange={(accent) => change({ accent })}
            />
          </Row>
          <Row label={t('shorts.second')}>
            <ColourField
              label={t('shorts.second')}
              value={style.second}
              onChange={(second) => change({ second })}
            />
          </Row>
        </Section>

        <Section
          id="shorts.header"
          title={t('shorts.header')}
          hint={t('shorts.headerHint')}
          actions={on(style.header.on, (next) => change({ header: { ...style.header, on: next } }))}
        >
          <Row label={t('shorts.call')}>
            <Input
              aria-label={t('shorts.call')}
              value={style.header.call}
              onChange={(event) =>
                change({ header: { ...style.header, call: event.target.value } })
              }
            />
          </Row>
          <Row label={t('shorts.font')}>
            <Input
              aria-label={t('shorts.font')}
              className="w-56"
              value={style.header.font}
              onChange={(event) =>
                change({ header: { ...style.header, font: event.target.value } })
              }
            />
          </Row>
          <Checkbox
            checked={style.header.title_upper}
            onCheckedChange={(title_upper) => change({ header: { ...style.header, title_upper } })}
          >
            {t('shorts.titleUpper')}
          </Checkbox>
        </Section>

        <Section
          id="shorts.sign"
          title={t('shorts.sign')}
          hint={t('shorts.signHint')}
          actions={on(style.sign.on, (next) => change({ sign: { ...style.sign, on: next } }))}
        >
          <Row label={t('shorts.signArt')}>
            <Art
              label={t('shorts.signArt')}
              value={style.sign.art}
              onChange={(art) => change({ sign: { ...style.sign, art } })}
            />
          </Row>
          <Row label={t('shorts.signBlink')}>
            <Art
              label={t('shorts.signBlink')}
              value={style.sign.blink}
              onChange={(blink) => change({ sign: { ...style.sign, blink } })}
            />
          </Row>
          <Row label={t('shorts.signSize')}>
            <Number
              label={t('shorts.signSize')}
              value={style.sign.size}
              min={80}
              max={400}
              step={10}
              unit={t('shorts.px')}
              onChange={(size) => change({ sign: { ...style.sign, size } })}
            />
          </Row>
          <Row label={t('shorts.signPulse')}>
            <Number
              label={t('shorts.signPulse')}
              value={Math.round(style.sign.pulse * 100)}
              min={0}
              max={60}
              step={2}
              unit="%"
              onChange={(percent) => change({ sign: { ...style.sign, pulse: percent / 100 } })}
            />
          </Row>
          <Row label={t('shorts.signSway')}>
            <Number
              label={t('shorts.signSway')}
              value={style.sign.sway}
              min={0}
              max={20}
              step={1}
              unit="°"
              onChange={(sway) => change({ sign: { ...style.sign, sway } })}
            />
          </Row>
        </Section>

        <Section id="shorts.ground" title={t('shorts.ground')} hint={t('shorts.groundHint')}>
          <Row label={t('shorts.groundColour')}>
            <ColourField
              label={t('shorts.groundColour')}
              value={style.ground.color}
              onChange={(color) => change({ ground: { ...style.ground, color } })}
            />
          </Row>
          <Ground
            pattern={style.ground.pattern}
            seed={style.ground.seed}
            blots={style.ground.blots}
            onChange={(patch) => change({ ground: { ...style.ground, ...patch } })}
          />
          <Checkbox
            checked={style.ground.motion}
            onCheckedChange={(motion) => change({ ground: { ...style.ground, motion } })}
          >
            {t('shorts.groundMotion')}
          </Checkbox>
        </Section>

        <Section
          id="shorts.stamp"
          title={t('shorts.stamp')}
          hint={t('shorts.stampHint')}
          actions={on(style.stamp.on, (next) => change({ stamp: { ...style.stamp, on: next } }))}
        >
          <Row label={t('shorts.stampShape')}>
            <Select
              aria-label={t('shorts.stampShape')}
              className="w-40"
              value={style.stamp.shape}
              options={(['stamp', 'plate'] as const).map((value) => ({
                value,
                label: t(`shorts.stampShapes.${value}`),
              }))}
              onChange={(shape) =>
                change({ stamp: { ...style.stamp, shape: shape === 'plate' ? 'plate' : 'stamp' } })
              }
            />
          </Row>
          <Row label={t('shorts.stampText')}>
            <Input
              aria-label={t('shorts.stampText')}
              value={style.stamp.text}
              onChange={(event) => change({ stamp: { ...style.stamp, text: event.target.value } })}
            />
          </Row>
          <Row label={t('shorts.stampSub')}>
            <Input
              aria-label={t('shorts.stampSub')}
              value={style.stamp.sub}
              onChange={(event) => change({ stamp: { ...style.stamp, sub: event.target.value } })}
            />
          </Row>
          <Row label={t('shorts.stampArt')}>
            <Art
              label={t('shorts.stampArt')}
              value={style.stamp.art}
              onChange={(art) => change({ stamp: { ...style.stamp, art } })}
            />
          </Row>
          <Row label={t('shorts.stampColour')}>
            <ColourField
              label={t('shorts.stampColour')}
              value={style.stamp.color}
              onChange={(color) => change({ stamp: { ...style.stamp, color } })}
            />
          </Row>
          <Row label={t('shorts.stampAngle')}>
            <Number
              label={t('shorts.stampAngle')}
              value={style.stamp.angle}
              min={-30}
              max={30}
              step={1}
              unit="°"
              onChange={(angle) => change({ stamp: { ...style.stamp, angle } })}
            />
          </Row>
          <Row label={t('shorts.stampX')}>
            <Number
              label={t('shorts.stampX')}
              value={style.stamp.x}
              min={0}
              max={1080}
              step={10}
              unit={t('shorts.px')}
              onChange={(x) => change({ stamp: { ...style.stamp, x } })}
            />
          </Row>
          <Row label={t('shorts.stampWear')}>
            <Number
              label={t('shorts.stampWear')}
              value={Math.round(style.stamp.wear * 100)}
              min={0}
              max={100}
              step={5}
              unit="%"
              onChange={(percent) => change({ stamp: { ...style.stamp, wear: percent / 100 } })}
            />
          </Row>
        </Section>

        <Section
          id="shorts.wave"
          title={t('shorts.wave')}
          hint={t('shorts.waveHint')}
          actions={on(style.wave.on, (next) => change({ wave: { ...style.wave, on: next } }))}
        >
          <Row label={t('shorts.waveKind')}>
            <Wave
              value={style.wave.kind}
              onChange={(kind) => change({ wave: { ...style.wave, kind } })}
            />
          </Row>
          <Checkbox
            checked={style.wave.strip}
            onCheckedChange={(strip) => change({ wave: { ...style.wave, strip } })}
          >
            {t('shorts.strip')}
          </Checkbox>
        </Section>

        <Section
          id="shorts.karaoke"
          title={t('shorts.karaoke')}
          hint={t('shorts.karaokeHint')}
          actions={on(style.karaoke.on, (next) =>
            change({ karaoke: { ...style.karaoke, on: next } }),
          )}
        >
          <Row label={t('shorts.font')}>
            <Input
              aria-label={t('shorts.font')}
              className="w-56"
              value={style.karaoke.font}
              onChange={(event) =>
                change({ karaoke: { ...style.karaoke, font: event.target.value } })
              }
            />
          </Row>
          <Row label={t('shorts.karaokeSize')}>
            <Number
              label={t('shorts.karaokeSize')}
              value={style.karaoke.size}
              min={30}
              max={110}
              step={2}
              unit={t('shorts.px')}
              onChange={(size) => change({ karaoke: { ...style.karaoke, size: Math.round(size) } })}
            />
          </Row>
        </Section>

        <Section id="shorts.effects" title={t('shorts.effects')} hint={t('shorts.effectsHint')}>
          <Row label={t('shorts.zoom')}>
            <Number
              label={t('shorts.zoom')}
              value={Math.round(style.effects.zoom * 100)}
              min={0}
              max={40}
              step={1}
              unit="%"
              onChange={(percent) => change({ effects: { ...style.effects, zoom: percent / 100 } })}
            />
          </Row>
          <Row label={t('shorts.grain')}>
            <Number
              label={t('shorts.grain')}
              value={style.effects.grain}
              min={0}
              max={40}
              step={1}
              onChange={(grain) =>
                change({ effects: { ...style.effects, grain: Math.round(grain) } })
              }
            />
          </Row>
          <Checkbox
            checked={style.effects.vignette}
            onCheckedChange={(vignette) => change({ effects: { ...style.effects, vignette } })}
          >
            {t('shorts.vignette')}
          </Checkbox>
          <Checkbox
            checked={style.effects.glitch}
            onCheckedChange={(glitch) => change({ effects: { ...style.effects, glitch } })}
          >
            {t('shorts.glitch')}
          </Checkbox>
        </Section>

        <Section
          id="shorts.tail"
          title={t('shorts.tail')}
          hint={t('shorts.tailHint')}
          actions={on(style.tail.on, (next) => change({ tail: { ...style.tail, on: next } }))}
        >
          <Row label={t('shorts.tailSeconds')}>
            <Number
              label={t('shorts.tailSeconds')}
              value={style.tail.seconds}
              min={0.3}
              max={5}
              step={0.1}
              unit={t('build.seconds')}
              onChange={(seconds) => change({ tail: { ...style.tail, seconds } })}
            />
          </Row>
        </Section>

        <Section
          title={t('shorts.variants')}
          hint={t('shorts.variantsHint')}
          actions={
            <Button
              size="xs"
              variant="ghost"
              onClick={() => {
                const key = freeKey(style)
                change({ variants: [...style.variants, { key, label: '' }] })
                setShown(key)
              }}
            >
              <Plus aria-hidden />
              {t('shorts.addVariant')}
            </Button>
          }
        >
          {style.variants.length === 0 ? (
            <p className="text-xs text-faint">{t('shorts.noVariants')}</p>
          ) : (
            <ChipGroup
              aria-label={t('shorts.variants')}
              value={shown === null ? [] : [shown]}
              onValueChange={(next) => setShown(next.find((key) => key !== shown) ?? null)}
            >
              {style.variants.map((entry) => (
                <Chip key={entry.key} value={entry.key}>
                  {entry.label?.trim() || entry.key}
                </Chip>
              ))}
            </ChipGroup>
          )}
          {variant !== undefined && (
            <VariantEditor
              // A variant of its own per key: what was typed for one must not
              // stand in the boxes of the next.
              key={variant.key}
              style={style}
              variant={variant}
              onChange={(patch) => setVariant(variant.key, patch)}
              onRename={(key) => {
                if (key === '' || style.variants.some((entry) => entry.key === key)) return
                setVariant(variant.key, { key })
                setShown(key)
              }}
              onRemove={() => {
                change({ variants: style.variants.filter((entry) => entry.key !== variant.key) })
                setShown(null)
              }}
            />
          )}
        </Section>
      </div>

      <aside className="sticky top-0 flex w-64 shrink-0 flex-col gap-2">
        <ShortFrame look={worn(style, variant)} title={t('shorts.sampleTitle')} />
        <p className="text-2xs text-faint">
          {variant === undefined
            ? t('shorts.shownStyle')
            : t('shorts.shownVariant', { name: variant.label?.trim() || variant.key })}
        </p>
      </aside>
    </div>
  )
}

/** A setting on its row: what it is called, then its control. */
function Row({ label, children }: { label: string; children: ReactNode }) {
  // A row, not a `<label>`: a label around a number field would forward a
  // click on its words to the stepper's minus.
  return (
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <span aria-hidden className="w-40 shrink-0 text-dim">
        {label}
      </span>
      {children}
    </div>
  )
}

/** A number of the style: an emptied box is a number being retyped, and the
 *  style keeps what it had until a number is there. */
function Number({
  label,
  value,
  onChange,
  ...rest
}: {
  label: string
  value: number
  min?: number
  max?: number
  step?: number
  unit?: ReactNode
  onChange: (value: number) => void
}) {
  return (
    <NumberField
      aria-label={label}
      className="w-36"
      value={value}
      onValueChange={(next) => {
        if (next !== null) onChange(next)
      }}
      {...rest}
    />
  )
}

/** A picture kept in the style as text: the text of an SVG, or a small
 *  picture as a `data:` address. */
function Art({
  label,
  value,
  onChange,
}: {
  label: string
  value: string
  onChange: (value: string) => void
}) {
  const { t } = useTranslation()
  const input = useRef<HTMLInputElement>(null)
  const src = artSrc(value)
  const read = async (file: File) => {
    const svg = file.type === 'image/svg+xml' || file.name.toLowerCase().endsWith('.svg')
    const text = svg
      ? await file.text()
      : await new Promise<string>((resolve, reject) => {
          const reader = new FileReader()
          reader.onload = () => resolve(String(reader.result))
          reader.onerror = () => reject(new Error(t('shorts.artUnreadable')))
          reader.readAsDataURL(file)
        })
    if (text.length > ART_LIMIT) {
      say.failed(new Error(t('shorts.artTooLarge')))
      return
    }
    onChange(text)
  }
  return (
    <span className="flex items-center gap-2">
      {src !== '' && (
        <img
          src={src}
          alt=""
          className="size-9 rounded-md border border-line bg-soft object-contain"
        />
      )}
      <input
        ref={input}
        type="file"
        accept=".svg,image/svg+xml,image/png,image/webp,image/jpeg"
        className="hidden"
        aria-label={t('shorts.chooseArt', { what: label })}
        onChange={(event) => {
          const file = event.target.files?.[0]
          event.target.value = ''
          if (file !== undefined) read(file).catch((cause: unknown) => say.failed(cause))
        }}
      />
      <Button size="xs" variant="ghost" onClick={() => input.current?.click()}>
        <Upload aria-hidden />
        {src === '' ? t('shorts.choose') : t('shorts.replace')}
      </Button>
      {src !== '' && (
        <Button size="xs" variant="ghost" onClick={() => onChange('')}>
          {t('shorts.remove')}
        </Button>
      )}
    </span>
  )
}

/** How the sound is drawn. */
function Wave({ value, onChange }: { value: WaveKind; onChange: (value: WaveKind) => void }) {
  const { t } = useTranslation()
  return (
    <Select
      aria-label={t('shorts.waveKind')}
      className="w-40"
      value={value}
      options={WAVES.map((kind) => ({ value: kind, label: t(`shorts.waves.${kind}`) }))}
      onChange={(next) => onChange(WAVES.find((kind) => kind === next) ?? 'solid')}
    />
  )
}

/** What a ground is covered with: the pattern, what it is drawn from, and -
 *  for blots - where each lies. The style's, and a variant's in place of it. */
function Ground({
  pattern,
  seed,
  blots,
  onChange,
}: {
  pattern: GroundPattern
  seed: number
  blots: ShortBlot[]
  onChange: (patch: { pattern?: GroundPattern; seed?: number; blots?: ShortBlot[] }) => void
}) {
  const { t } = useTranslation()
  const setBlot = (at: number, patch: Partial<ShortBlot>) => {
    onChange({ blots: blots.map((blot, index) => (index === at ? { ...blot, ...patch } : blot)) })
  }
  return (
    <>
      <Row label={t('shorts.pattern')}>
        <Select
          aria-label={t('shorts.pattern')}
          className="w-40"
          value={pattern}
          options={PATTERNS.map((value) => ({ value, label: t(`shorts.patterns.${value}`) }))}
          onChange={(next) =>
            onChange({ pattern: PATTERNS.find((value) => value === next) ?? 'plain' })
          }
        />
        {pattern !== 'plain' && (
          <Button
            size="xs"
            variant="ghost"
            title={t('shorts.reshuffleHint')}
            onClick={() => onChange({ seed: Math.floor(Math.random() * 1_000_000) })}
          >
            <Shuffle aria-hidden />
            {t('shorts.reshuffle')}
          </Button>
        )}
        <span className="font-mono text-2xs text-faint">{t('shorts.seed', { seed })}</span>
      </Row>
      {pattern === 'blots' && (
        <ul className="flex flex-col gap-1.5">
          {blots.map((blot, at) => (
            <li key={at} className="flex flex-wrap items-center gap-1.5">
              <Number
                label={t('shorts.blotX', { n: at + 1 })}
                value={blot.x}
                step={10}
                onChange={(x) => setBlot(at, { x })}
              />
              <Number
                label={t('shorts.blotY', { n: at + 1 })}
                value={blot.y}
                step={10}
                onChange={(y) => setBlot(at, { y })}
              />
              <Number
                label={t('shorts.blotSize', { n: at + 1 })}
                value={blot.size}
                min={100}
                max={2000}
                step={20}
                onChange={(size) => setBlot(at, { size })}
              />
              <Select
                aria-label={t('shorts.blotColour', { n: at + 1 })}
                className="w-32"
                value={blot.color ?? 'accent'}
                options={(['accent', 'second'] as const).map((value) => ({
                  value,
                  label: t(`shorts.${value}`),
                }))}
                onChange={(next) => setBlot(at, { color: next === 'second' ? 'second' : 'accent' })}
              />
              <Select
                aria-label={t('shorts.blotShape', { n: at + 1 })}
                className="w-28"
                value={blot.shape ?? 'splat'}
                options={SHAPES.map((value) => ({ value, label: t(`shorts.shapes.${value}`) }))}
                onChange={(next) =>
                  setBlot(at, { shape: SHAPES.find((value) => value === next) ?? 'splat' })
                }
              />
              <Button
                variant="icon"
                size="icon-sm"
                aria-label={t('shorts.removeBlot', { n: at + 1 })}
                onClick={() => onChange({ blots: blots.filter((_, index) => index !== at) })}
              >
                <Trash2 aria-hidden />
              </Button>
            </li>
          ))}
          <li>
            <Button
              size="xs"
              variant="ghost"
              onClick={() =>
                onChange({
                  blots: [...blots, { x: 540, y: 960, size: 800, color: 'accent', shape: 'splat' }],
                })
              }
            >
              <Plus aria-hidden />
              {t('shorts.addBlot')}
            </Button>
          </li>
        </ul>
      )}
    </>
  )
}

/** One variant: what it replaces of the style, each with a box that says
 *  whether it replaces it at all. */
function VariantEditor({
  style,
  variant,
  onChange,
  onRename,
  onRemove,
}: {
  style: ShortStyle
  variant: ShortVariant
  onChange: (patch: Partial<ShortVariant>) => void
  onRename: (key: string) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  const [key, setKey] = useState(variant.key)
  /** A value of the variant's own, with the box that gives it back to the style. */
  const own = (
    label: string,
    has: boolean,
    take: () => void,
    give: () => void,
    control: ReactNode,
  ) => (
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <span className="w-40 shrink-0">
        <Checkbox checked={has} onCheckedChange={(next) => (next ? take() : give())}>
          {label}
        </Checkbox>
      </span>
      {has ? control : <span className="text-xs text-faint">{t('shorts.asStyle')}</span>}
    </div>
  )
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-line p-3">
      <Row label={t('shorts.variantKey')}>
        <Input
          aria-label={t('shorts.variantKey')}
          className="w-40 font-mono"
          value={key}
          onChange={(event) => setKey(event.target.value.trim())}
          onBlur={() => {
            if (key !== variant.key) onRename(key)
          }}
        />
      </Row>
      <Row label={t('shorts.variantLabel')}>
        <Input
          aria-label={t('shorts.variantLabel')}
          className="w-56"
          value={variant.label ?? ''}
          onChange={(event) => onChange({ label: event.target.value })}
        />
      </Row>
      {own(
        t('shorts.accent'),
        variant.accent != null,
        () => onChange({ accent: style.accent }),
        () => onChange({ accent: undefined }),
        <ColourField
          label={t('shorts.accent')}
          value={variant.accent ?? style.accent}
          onChange={(accent) => onChange({ accent })}
        />,
      )}
      {own(
        t('shorts.second'),
        variant.second != null,
        () => onChange({ second: style.second }),
        () => onChange({ second: undefined }),
        <ColourField
          label={t('shorts.second')}
          value={variant.second ?? style.second}
          onChange={(second) => onChange({ second })}
        />,
      )}
      {own(
        t('shorts.signArt'),
        variant.sign_art != null,
        () => onChange({ sign_art: style.sign.art }),
        () => onChange({ sign_art: undefined, sign_blink: undefined }),
        <span className="flex flex-wrap items-center gap-3">
          <Art
            label={t('shorts.signArt')}
            value={variant.sign_art ?? ''}
            onChange={(sign_art) => onChange({ sign_art })}
          />
          <span className="text-xs text-dim">{t('shorts.signBlink')}</span>
          <Art
            label={t('shorts.signBlink')}
            value={variant.sign_blink ?? ''}
            onChange={(sign_blink) =>
              onChange({ sign_blink: sign_blink === '' ? undefined : sign_blink })
            }
          />
        </span>,
      )}
      {own(
        t('shorts.ground'),
        variant.ground != null,
        () =>
          onChange({
            ground: {
              pattern: style.ground.pattern,
              seed: style.ground.seed,
              blots: style.ground.blots,
            },
          }),
        () => onChange({ ground: undefined }),
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          {variant.ground != null && (
            <Ground
              pattern={variant.ground.pattern}
              seed={variant.ground.seed}
              blots={variant.ground.blots}
              onChange={(patch) => {
                if (variant.ground != null) onChange({ ground: { ...variant.ground, ...patch } })
              }}
            />
          )}
        </div>,
      )}
      {own(
        t('shorts.stampAngle'),
        variant.stamp_angle != null,
        () => onChange({ stamp_angle: style.stamp.angle }),
        () => onChange({ stamp_angle: undefined }),
        <Number
          label={t('shorts.stampAngle')}
          value={variant.stamp_angle ?? style.stamp.angle}
          min={-30}
          max={30}
          step={1}
          unit="°"
          onChange={(stamp_angle) => onChange({ stamp_angle })}
        />,
      )}
      {own(
        t('shorts.stampX'),
        variant.stamp_x != null,
        () => onChange({ stamp_x: style.stamp.x }),
        () => onChange({ stamp_x: undefined }),
        <Number
          label={t('shorts.stampX')}
          value={variant.stamp_x ?? style.stamp.x}
          min={0}
          max={1080}
          step={10}
          unit={t('shorts.px')}
          onChange={(stamp_x) => onChange({ stamp_x })}
        />,
      )}
      {own(
        t('shorts.waveKind'),
        variant.wave != null,
        () => onChange({ wave: style.wave.kind }),
        () => onChange({ wave: undefined }),
        <Wave value={variant.wave ?? style.wave.kind} onChange={(wave) => onChange({ wave })} />,
      )}
      <Button size="xs" variant="ghost" className="self-start text-bad" onClick={onRemove}>
        <Trash2 aria-hidden />
        {t('shorts.removeVariant')}
      </Button>
    </div>
  )
}
