import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronDown, ChevronRight, Plus, X } from 'lucide-react'
import type { Axis, AxisKind, AxisMark, AxisOption } from '@/lib/api/types'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { NumberField } from '@/components/ui/number-field'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'

/** An axis newly made: a scale of ten at the weight of one, as the shipped
 *  crafts' axes are. */
function newAxis(key: string): Axis {
  return { key, label: key, weight: 1, scale: 10, kind: 'scale' }
}

/**
 * A kind's scoring axes, whole (v0.112): the word, the weight, the scale and
 * the kind of answer each takes, and behind a row's arrow what the axis asks,
 * the answers of a choice and the rubric - what a mark on it means.
 *
 * The key is shown and stands: past score snapshots are stored under it. An
 * axis can be removed; its old scores stay where they were, read under the
 * key.
 */
export function AxesEditor({
  axes,
  onChange,
  problems,
}: {
  axes: Axis[]
  onChange: (axes: Axis[]) => void
  problems?: (key: string) => readonly string[]
}) {
  const { t } = useTranslation()
  const [open, setOpen] = useState<readonly string[]>([])

  const kinds: { value: AxisKind; label: string }[] = [
    { value: 'scale', label: t('editor.axisKind.scale') },
    { value: 'flag', label: t('editor.axisKind.flag') },
    { value: 'choice', label: t('editor.axisKind.choice') },
  ]

  return (
    <KeyedList
      label={t('editor.axes')}
      help={t('editor.axesHint')}
      entries={axes}
      onChange={onChange}
      make={newAxis}
      addLabel={t('editor.addAxis')}
      removeLabel={() => t('editor.removeAxis')}
      problems={problems}
      below={(axis, set) =>
        open.includes(axis.key) ? (
          <div className="flex flex-col gap-3 border-l border-line pl-3">
            <Field label={t('editor.axisDescription')} help={t('editor.axisDescriptionHint')}>
              <LabelInput
                optional
                label={axis.description}
                onChange={(description) => set({ description })}
              />
            </Field>
            {axis.kind === 'choice' && (
              <KeyedList<AxisOption>
                label={t('editor.axisOptions')}
                help={t('editor.axisOptionsHint', { scale: axis.scale })}
                entries={axis.options ?? []}
                onChange={(options) => set({ options })}
                make={(key) => ({ key, label: key, value: 0 })}
              >
                {(option, setOption) => (
                  <>
                    <LabelInput
                      className="flex-1"
                      label={option.label}
                      onChange={(label) => setOption({ label: label ?? '' })}
                      aria-label={t('editor.wordOf', { key: option.key })}
                    />
                    <NumberField
                      className="w-24"
                      min={0}
                      max={axis.scale}
                      step={0.5}
                      value={option.value}
                      onValueChange={(value) => setOption({ value: value ?? 0 })}
                      aria-label={t('editor.axisOptionValue', { key: option.key })}
                    />
                  </>
                )}
              </KeyedList>
            )}
            <RubricEditor
              axis={axis}
              onChange={(rubric) => set({ rubric: rubric.length === 0 ? undefined : rubric })}
            />
          </div>
        ) : null
      }
    >
      {(axis, set) => (
        <>
          <Button
            variant="icon"
            size="icon-sm"
            aria-expanded={open.includes(axis.key)}
            aria-label={t('editor.axisDetails', { key: axis.key })}
            title={t('editor.axisDetails', { key: axis.key })}
            onClick={() =>
              setOpen(
                open.includes(axis.key)
                  ? open.filter((key) => key !== axis.key)
                  : [...open, axis.key],
              )
            }
          >
            {open.includes(axis.key) ? <ChevronDown aria-hidden /> : <ChevronRight aria-hidden />}
          </Button>
          <LabelInput
            className="flex-1"
            label={axis.label}
            onChange={(label) => set({ label: label ?? '' })}
            aria-label={`${axis.key} label`}
          />
          <NumberField
            className="w-36"
            min={0}
            step={0.5}
            value={axis.weight}
            // An emptied box is no weight, as it was when the box held text.
            onValueChange={(weight) => set({ weight: weight ?? 0 })}
            aria-label={`${axis.key} weight`}
            unit={t('editor.axisWeight')}
          />
          <NumberField
            className="w-28"
            hideStepper
            min={1}
            step={1}
            value={axis.scale}
            // No scale is no axis: an emptied box keeps the scale it had.
            onValueChange={(scale) => {
              if (scale !== null && scale > 0) set({ scale })
            }}
            aria-label={`${axis.key} scale`}
            unit={t('editor.axisScale')}
          />
          <Select
            className="w-36"
            aria-label={t('editor.axisKindOf', { key: axis.key })}
            value={axis.kind}
            onChange={(kind) =>
              set({
                kind: kind as AxisKind,
                // The answers belong to a choice alone, and the profile is
                // refused with them on any other kind.
                options: kind === 'choice' ? (axis.options ?? []) : undefined,
              })
            }
            options={kinds}
          />
        </>
      )}
    </KeyedList>
  )
}

/**
 * What the marks of an axis mean, for the marks worth naming: three or so on
 * a scale of ten. A mark with no sentence of its own reads the nearest named
 * mark below it.
 */
function RubricEditor({ axis, onChange }: { axis: Axis; onChange: (rubric: AxisMark[]) => void }) {
  const { t } = useTranslation()
  const rubric = axis.rubric ?? []

  const set = (index: number, changes: Partial<AxisMark>) => {
    onChange(rubric.map((mark, i) => (i === index ? { ...mark, ...changes } : mark)))
  }
  // The next mark worth naming: the first whole one the rubric does not have,
  // from the top of the scale down.
  const next = () => {
    for (let at = Math.floor(axis.scale); at >= 0; at -= 1) {
      if (!rubric.some((mark) => mark.at === at)) return at
    }
    return 0
  }

  return (
    <div className="flex flex-col gap-1.5">
      <span className="caption">{t('editor.rubric')}</span>
      <ul className="flex flex-col gap-1.5">
        {rubric.map((mark, index) => (
          // By place: a mark is named by its number, which is what is edited.
          <li key={index} className="flex items-center gap-2">
            <NumberField
              className="w-20"
              hideStepper
              min={0}
              max={axis.scale}
              step={1}
              value={mark.at}
              onValueChange={(at) => set(index, { at: at ?? 0 })}
              aria-label={t('editor.rubricAt')}
            />
            <LabelInput
              className="flex-1"
              label={mark.label}
              onChange={(label) => set(index, { label: label ?? '' })}
              aria-label={t('editor.rubricMeans', { at: mark.at })}
            />
            <Button
              variant="danger"
              size="icon-sm"
              title={t('editor.rubricRemove')}
              aria-label={t('editor.rubricRemove')}
              onClick={() => onChange(rubric.filter((_, i) => i !== index))}
            >
              <X aria-hidden className="size-3.5" />
            </Button>
          </li>
        ))}
      </ul>
      <Button
        size="sm"
        className="self-start"
        onClick={() => onChange([...rubric, { at: next(), label: '' }])}
      >
        <Plus aria-hidden />
        {t('editor.rubricAdd')}
      </Button>
      <p className="text-xs text-dim">{t('editor.rubricHint')}</p>
    </div>
  )
}
