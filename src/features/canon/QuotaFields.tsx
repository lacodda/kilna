import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { QuotaMetric, QuotaScope } from '@/lib/api/types'
import { METRICS, OPS, SCOPES, quotaFields, type QuotaDraft, type QuotaOp } from '@/lib/quota'
import { useProfile } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import {
  Combobox,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  draft: QuotaDraft
  onChange: (next: QuotaDraft) => void
}

/**
 * The boxes of a rule of a quotas section (v0.151, FR-20 20.2): the field it
 * measures, what it measures and against what, the works it reads, and how
 * loud a breach is. The rule's name is the fact's words, above.
 */
export function QuotaFields({ draft, onChange }: Props) {
  const { t } = useTranslation()
  const set = (patch: Partial<QuotaDraft>) => onChange({ ...draft, ...patch })
  const forbidden = draft.metric === 'forbidden_terms'
  const patterned = draft.metric === 'match_count' || draft.metric === 'share'

  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-2">
        <FieldName value={draft.field} onChange={(field) => set({ field })} />
        <Select
          value={draft.metric}
          onChange={(next) => {
            const metric = next as QuotaMetric
            // A share is of many works: one alone is not a set to take a
            // part of.
            set({
              metric,
              ...(metric === 'share' && draft.scope === 'item'
                ? { scope: 'collection_period' as const }
                : {}),
            })
          }}
          options={METRICS.map((metric) => ({
            value: metric,
            label: t(`canon.quota.metric.${metric}`),
          }))}
          aria-label={t('canon.quota.metricLabel')}
        />
      </div>

      {forbidden ? (
        <Textarea
          autoResize
          rows={1}
          value={draft.terms}
          onChange={(event) => set({ terms: event.target.value })}
          placeholder={t('canon.quota.termsPlaceholder')}
          aria-label={t('canon.quota.terms')}
          className="text-sm"
        />
      ) : (
        <div className="flex flex-wrap items-center gap-2">
          <Select
            value={draft.op}
            onChange={(next) => set({ op: next as QuotaOp })}
            options={OPS.map((op) => ({ value: op, label: t(`canon.quota.op.${op}`) }))}
            aria-label={t('canon.quota.opLabel')}
            className="w-36"
          />
          <NumberField
            className="w-28"
            min={0}
            max={draft.metric === 'share' ? 100 : undefined}
            value={draft.low}
            unit={draft.metric === 'share' ? '%' : undefined}
            aria-label={t(draft.op === 'between' ? 'canon.quota.low' : 'canon.quota.value')}
            onValueChange={(low) => set({ low })}
          />
          {draft.op === 'between' && (
            <>
              <span className="text-faint">-</span>
              <NumberField
                className="w-28"
                min={0}
                max={draft.metric === 'share' ? 100 : undefined}
                value={draft.high}
                unit={draft.metric === 'share' ? '%' : undefined}
                aria-label={t('canon.quota.high')}
                onValueChange={(high) => set({ high })}
              />
            </>
          )}
        </div>
      )}

      {patterned && (
        <Input
          value={draft.pattern}
          onChange={(event) => set({ pattern: event.target.value })}
          placeholder={t(
            draft.metric === 'share'
              ? 'canon.quota.patternOptional'
              : 'canon.quota.patternPlaceholder',
          )}
          aria-label={t('canon.quota.pattern')}
          className="font-mono text-xs"
        />
      )}

      <div className="flex flex-wrap items-center gap-3">
        <SegmentedControl
          aria-label={t('canon.quota.scopeLabel')}
          value={draft.scope}
          onValueChange={(next) => set({ scope: next as QuotaScope })}
        >
          {SCOPES.map((scope) => (
            <Segment
              key={scope}
              value={scope}
              disabled={scope === 'item' && draft.metric === 'share'}
            >
              {t(`canon.quota.scope.${scope}`)}
            </Segment>
          ))}
        </SegmentedControl>
        <SegmentedControl
          aria-label={t('canon.quota.severityLabel')}
          value={draft.severity}
          onValueChange={(next) => set({ severity: next === 'error' ? 'error' : 'warn' })}
        >
          <Segment value="warn">{t('canon.quota.severity.warn')}</Segment>
          <Segment value="error">{t('canon.quota.severity.error')}</Segment>
        </SegmentedControl>
        <Switch checked={draft.active} onCheckedChange={(active) => set({ active })}>
          {t('canon.quota.active')}
        </Switch>
      </div>

      {/* Only some works: the ones whose field says one of these. */}
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs text-faint">{t('canon.quota.onlyWhere')}</span>
        <Input
          value={draft.filterField}
          onChange={(event) => set({ filterField: event.target.value })}
          placeholder={t('canon.quota.filterFieldPlaceholder')}
          aria-label={t('canon.quota.filterField')}
          className="w-36 font-mono text-xs"
        />
        <Switch checked={draft.filterNot} onCheckedChange={(filterNot) => set({ filterNot })}>
          {t('canon.quota.filterNot')}
        </Switch>
        <Input
          value={draft.filterEquals}
          onChange={(event) => set({ filterEquals: event.target.value })}
          placeholder={t('canon.quota.filterEqualsPlaceholder')}
          aria-label={t('canon.quota.filterEquals')}
          className="min-w-40 flex-1 text-xs"
        />
      </div>
    </div>
  )
}

/** The field a rule measures: any word, with the profile's fields offered
 *  as it is typed. */
function FieldName({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const [open, setOpen] = useState(false)
  const typed = value.trim().toLowerCase()
  const offered = quotaFields(config).filter(
    (field) => field.toLowerCase() !== typed && field.toLowerCase().includes(typed),
  )
  return (
    <Combobox
      items={offered}
      filter={null}
      inputValue={value}
      onInputValueChange={onChange}
      onValueChange={(next) => {
        if (typeof next === 'string') onChange(next)
      }}
      open={open && offered.length > 0}
      onOpenChange={setOpen}
    >
      <ComboboxInput
        placeholder={t('canon.quota.fieldPlaceholder')}
        aria-label={t('canon.quota.field')}
        className="font-mono text-xs"
        onFocus={() => setOpen(true)}
      />
      <ComboboxPopup className="p-1">
        <ComboboxList>
          {(field: string) => (
            <ComboboxItem
              key={field}
              value={field}
              className="rounded-md px-2 py-1 font-mono text-xs"
            >
              {field}
            </ComboboxItem>
          )}
        </ComboboxList>
      </ComboboxPopup>
    </Combobox>
  )
}
