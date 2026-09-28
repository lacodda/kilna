import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import type { Axis } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'

/**
 * A kind's scoring axes: the label and the weight of each.
 *
 * The key is shown and not editable - past score snapshots are stored under
 * it, and renaming one would orphan its history. An axis can be removed; its
 * old scores stay where they were.
 */
export function AxesEditor({ axes, onChange }: { axes: Axis[]; onChange: (axes: Axis[]) => void }) {
  const { t } = useTranslation()

  const setAxis = (index: number, changes: Partial<Axis>) => {
    onChange(axes.map((axis, i) => (i === index ? { ...axis, ...changes } : axis)))
  }

  return (
    <FieldGroup label={t('editor.axes')} help={t('editor.axesHint')}>
      <ul className="flex flex-col gap-1.5">
        {axes.map((axis, index) => (
          <li key={axis.key} className="flex items-center gap-2">
            <code className="w-28 shrink-0 font-mono text-xs text-dim">{axis.key}</code>
            <Input
              className="flex-1"
              value={sayLabel(axis.label)}
              onChange={(event) => setAxis(index, { label: event.target.value })}
              aria-label={`${axis.key} label`}
            />
            <NumberField
              className="w-28"
              min={0}
              step={0.5}
              value={axis.weight}
              // An emptied box is no weight, as it was when the box held text.
              onValueChange={(weight) => setAxis(index, { weight: weight ?? 0 })}
              aria-label={`${axis.key} weight`}
            />
            <Button
              variant="danger"
              size="icon-sm"
              title={t('editor.removeAxis')}
              aria-label={t('editor.removeAxis')}
              onClick={() => onChange(axes.filter((_, i) => i !== index))}
            >
              <X aria-hidden className="size-3.5" />
            </Button>
          </li>
        ))}
      </ul>
    </FieldGroup>
  )
}
