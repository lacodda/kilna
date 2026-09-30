import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Link2, X } from 'lucide-react'
import { addFact, updateFact } from '@/lib/api/canon'
import type {
  CardView,
  Fact,
  FactPatch,
  FactSource,
  FactSourceKind,
  FactStatus,
  Layer,
  SectionShape,
  WorldTime,
} from '@/lib/api/types'
import type { JsonValue } from '@/lib/api/generated/serde_json/JsonValue'
import { cardKindOf, LAYERS, SETTLED, writableSections } from '@/lib/canon'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { Select } from '@/components/AppSelect'
import { PickWorkDialog } from '@/components/PickWorkDialog'

/** Where a signature detail may act; the backend holds the same three. */
const PLACES = ['cover', 'frame', 'scene'] as const

interface Props {
  view: CardView
  /** The section a new fact goes under; for an edit, the one it stands in. */
  section: string
  shape: SectionShape
  /** The fact being edited; none for a new one. */
  fact?: Fact
  onDone: () => void
}

/**
 * A fact being written: its words, its layer, how settled it is, when it
 * happened, where it came from - and what a section of another shape carries
 * beside the words. Saved as one gesture when the person says so; Escape
 * leaves it as it was.
 */
export function FactEditor({ view, section, shape, fact, onDone }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const kind = cardKindOf(config, view.card.kind)
  const text = (key: string) => {
    const value = fact?.data[key]
    return typeof value === 'string' ? value : ''
  }

  const [body, setBody] = useState(fact?.body ?? '')
  const [layer, setLayer] = useState<Layer>(fact?.layer ?? 'public')
  const [status, setStatus] = useState<FactStatus>(
    fact !== undefined && fact.status !== 'retired' ? fact.status : 'canon',
  )
  const [inSection, setInSection] = useState(section)
  const [whenLabel, setWhenLabel] = useState(fact?.when?.label ?? '')
  const [whenSort, setWhenSort] = useState(fact?.when?.sort ?? '')
  const [sourceKind, setSourceKind] = useState<FactSourceKind | ''>(fact?.source?.kind ?? '')
  const [sourceWork, setSourceWork] = useState<{ id: string; title: string } | null>(
    fact?.source?.kind === 'work' && fact.source.work_id
      ? { id: fact.source.work_id, title: fact.source.label ?? '' }
      : null,
  )
  const [sourceLine, setSourceLine] = useState(fact?.source?.line ?? '')
  const [sourceLabel, setSourceLabel] = useState(
    fact?.source?.kind === 'work' ? '' : (fact?.source?.label ?? ''),
  )
  const [picking, setPicking] = useState(false)
  const [slot, setSlot] = useState(text('slot'))
  const [template, setTemplate] = useState(text('template'))
  const [places, setPlaces] = useState<string[]>(
    Array.isArray(fact?.data.places) ? (fact.data.places as string[]) : [...PLACES],
  )
  const [on, setOn] = useState(fact?.data.on !== false)
  const [color, setColor] = useState(text('color'))
  const [code, setCode] = useState(text('code'))
  const [prompt, setPrompt] = useState(text('prompt'))
  const [styleId, setStyleId] = useState(text('styleId'))
  const bricks = useQuery({ ...queries.styleBricksMatching(null, ''), enabled: shape === 'styles' })

  const source = (): FactSource | null => {
    switch (sourceKind) {
      case 'work':
        return sourceWork === null
          ? null
          : {
              kind: 'work',
              work_id: sourceWork.id,
              line: sourceLine.trim() === '' ? undefined : sourceLine.trim(),
              label: sourceWork.title,
            }
      case 'decision':
      case 'document':
        return sourceLabel.trim() === '' ? null : { kind: sourceKind, label: sourceLabel.trim() }
      default:
        return null
    }
  }
  const when = (): WorldTime | null =>
    whenLabel.trim() === '' && whenSort.trim() === ''
      ? null
      : {
          label: whenLabel.trim() === '' ? undefined : whenLabel.trim(),
          sort: whenSort.trim() === '' ? undefined : whenSort.trim(),
        }
  const data = (): Record<string, JsonValue> => {
    switch (shape) {
      case 'slots':
        return { slot: slot.trim() }
      case 'details':
        return { template: template.trim(), places, on }
      case 'palette':
        return { color: color.trim() }
      case 'marks':
        return {
          ...(code.trim() !== '' ? { code: code.trim() } : {}),
          ...(prompt.trim() !== '' ? { prompt: prompt.trim() } : {}),
        }
      case 'styles':
        return { styleId }
      default:
        return {}
    }
  }
  const words = (): string => {
    if (shape === 'styles' && body.trim() === '') {
      return (bricks.data ?? []).find((one) => one.id === styleId)?.name ?? ''
    }
    return body.trim()
  }

  const save = useAppMutation({
    mutationFn: () => {
      if (fact === undefined) {
        return addFact({
          note_id: view.card.id,
          section: inSection,
          body: words(),
          layer,
          status,
          when: when() ?? undefined,
          source: source() ?? undefined,
          data: data(),
        })
      }
      const patch: FactPatch = {
        body: words(),
        layer,
        status,
        when: when(),
        source: source(),
        data: data(),
      }
      if (inSection !== fact.section) patch.section = inSection
      return updateFact(fact.id, patch)
    },
    failure: 'toast.factSaveFailed',
    refresh: refresh.canon,
    onSuccess: onDone,
  })

  const ready =
    words() !== '' &&
    (shape !== 'slots' || slot.trim() !== '') &&
    (shape !== 'details' || template.trim() !== '') &&
    (shape !== 'styles' || styleId !== '') &&
    (sourceKind !== 'work' || sourceWork !== null) &&
    ((sourceKind !== 'decision' && sourceKind !== 'document') || sourceLabel.trim() !== '')

  const sections = kind === undefined ? [] : writableSections(kind)

  return (
    <div
      className="my-1 flex flex-col gap-2.5 rounded-lg border border-line-2 bg-bg/60 p-2.5"
      onKeyDown={(event) => {
        if (event.key === 'Escape' && !event.defaultPrevented) {
          event.preventDefault()
          onDone()
        }
        if ((event.ctrlKey || event.metaKey) && event.key === 'Enter' && ready) save.mutate()
      }}
    >
      {shape === 'styles' ? (
        <Select
          value={styleId}
          onChange={setStyleId}
          placeholder={t('canon.pickStyle')}
          options={(bricks.data ?? []).map((one) => ({ value: one.id, label: one.name }))}
          aria-label={t('canon.pickStyle')}
        />
      ) : null}
      <Textarea
        autoFocus
        autoResize
        rows={shape === 'facts' ? 2 : 1}
        value={body}
        onChange={(event) => setBody(event.target.value)}
        placeholder={t(`canon.placeholder.${shape}`)}
        aria-label={t('canon.factWords')}
        className="text-sm leading-relaxed"
      />

      {shape === 'slots' && (
        <Input
          value={slot}
          onChange={(event) => setSlot(event.target.value)}
          placeholder={t('canon.slotPlaceholder')}
          aria-label={t('canon.slot')}
          className="font-mono"
        />
      )}
      {shape === 'details' && (
        <>
          <Textarea
            autoResize
            rows={2}
            value={template}
            onChange={(event) => setTemplate(event.target.value)}
            placeholder={t('canon.templatePlaceholder')}
            aria-label={t('canon.template')}
            className="font-mono text-xs"
          />
          <div className="flex flex-wrap items-center gap-3">
            <ChipGroup
              multiple
              aria-label={t('canon.places')}
              value={places}
              onValueChange={(next) => setPlaces(next)}
            >
              {PLACES.map((place) => (
                <Chip key={place} value={place}>
                  {t(`canon.place.${place}`)}
                </Chip>
              ))}
            </ChipGroup>
            <Switch checked={on} onCheckedChange={setOn}>
              {t('canon.detailOnByDefault')}
            </Switch>
          </div>
        </>
      )}
      {shape === 'palette' && (
        <div className="flex items-center gap-2">
          <input
            type="color"
            value={/^#[0-9a-fA-F]{6}$/.test(color) ? color : ''}
            onChange={(event) => setColor(event.target.value.toUpperCase())}
            aria-label={t('canon.color')}
            className="h-8 w-10 cursor-pointer rounded-md border border-line bg-transparent"
          />
          <Input
            value={color}
            onChange={(event) => setColor(event.target.value)}
            aria-label={t('canon.color')}
            className="w-28 font-mono"
          />
        </div>
      )}
      {shape === 'marks' && (
        <div className="grid grid-cols-[120px_minmax(0,1fr)] gap-2">
          <Input
            value={code}
            onChange={(event) => setCode(event.target.value)}
            placeholder={t('canon.codePlaceholder')}
            aria-label={t('canon.code')}
            className="font-mono"
          />
          <Input
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            placeholder={t('canon.markPromptPlaceholder')}
            aria-label={t('canon.markPrompt')}
          />
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          aria-label={t('canon.layerLabel')}
          value={layer}
          onValueChange={(next) => setLayer(next as Layer)}
        >
          {LAYERS.map((one) => (
            <Segment key={one} value={one}>
              {t(`canon.layer.short.${one}`)} · {t(`canon.layer.name.${one}`)}
            </Segment>
          ))}
        </SegmentedControl>
        <Select
          value={status}
          onChange={(next) => setStatus(next as FactStatus)}
          options={SETTLED.map((one) => ({ value: one, label: t(`canon.status.${one}`) }))}
          aria-label={t('canon.statusLabel')}
          className="w-36"
        />
        {fact !== undefined && sections.length > 1 && (
          <Select
            value={inSection}
            onChange={setInSection}
            options={sections.map((one) => ({ value: one.key, label: sayLabel(one.label) }))}
            aria-label={t('canon.moveTo')}
            className="w-44"
          />
        )}
      </div>

      <div className="grid grid-cols-[minmax(0,1fr)_140px] gap-2">
        <Input
          value={whenLabel}
          onChange={(event) => setWhenLabel(event.target.value)}
          placeholder={t('canon.whenPlaceholder')}
          aria-label={t('canon.when')}
        />
        <Input
          value={whenSort}
          onChange={(event) => setWhenSort(event.target.value)}
          placeholder={t('canon.sortPlaceholder')}
          aria-label={t('canon.sort')}
          title={t('canon.sortHint')}
          className="font-mono"
        />
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Select
          value={sourceKind}
          onChange={(next) => setSourceKind(next as FactSourceKind | '')}
          options={[
            { value: '', label: t('canon.source.none') },
            { value: 'work', label: t('canon.source.work') },
            { value: 'decision', label: t('canon.source.decision') },
            { value: 'document', label: t('canon.source.document') },
          ]}
          aria-label={t('canon.sourceLabel')}
          className="w-40"
        />
        {sourceKind === 'work' && (
          <>
            {sourceWork === null ? (
              <Button size="sm" variant="ghost" onClick={() => setPicking(true)}>
                <Link2 aria-hidden />
                {t('canon.pickWork')}
              </Button>
            ) : (
              <Chip onRemove={() => setSourceWork(null)} removeLabel={t('canon.dropWork')}>
                {sourceWork.title}
              </Chip>
            )}
            <Input
              value={sourceLine}
              onChange={(event) => setSourceLine(event.target.value)}
              placeholder={t('canon.linePlaceholder')}
              aria-label={t('canon.line')}
              className="min-w-48 flex-1"
            />
          </>
        )}
        {(sourceKind === 'decision' || sourceKind === 'document') && (
          <Input
            value={sourceLabel}
            onChange={(event) => setSourceLabel(event.target.value)}
            placeholder={t(`canon.sourcePlaceholder.${sourceKind}`)}
            aria-label={t('canon.sourceName')}
            className="min-w-48 flex-1"
          />
        )}
      </div>

      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="primary"
          disabled={!ready || save.isPending}
          onClick={() => save.mutate()}
        >
          {fact === undefined ? t('canon.addFact') : t('canon.saveFact')}
        </Button>
        <Button size="sm" variant="ghost" onClick={onDone}>
          <X aria-hidden />
          {t('dialog.cancel')}
        </Button>
        <span className="ml-auto text-2xs text-faint">{t('canon.editorKeys')}</span>
      </div>

      {picking && (
        <PickWorkDialog
          open
          onOpenChange={setPicking}
          title={t('canon.pickWork')}
          onPick={(work) => {
            setPicking(false)
            setSourceWork({ id: work.work_id, title: work.title })
          }}
        />
      )}
    </div>
  )
}
