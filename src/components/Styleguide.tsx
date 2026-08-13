import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Pencil, Plus, Star, Trash2, Undo2, X } from 'lucide-react'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { DatePicker } from '@/components/ui/DatePicker'
import { Field, Input, Textarea } from '@/components/ui/Input'
import { Panel, SectionLabel } from '@/components/ui/Panel'
import { Select } from '@/components/ui/Select'

const TOKENS = [
  'bg',
  'raise',
  'soft',
  'softer',
  'line',
  'line-2',
  'text',
  'dim',
  'faint',
  'accent',
  'accent-2',
  'accent-soft',
  'good',
  'good-soft',
  'warn',
  'warn-soft',
  'bad',
  'bad-soft',
  'info',
  'info-soft',
] as const

const BADGES = ['outline', 'accent', 'good', 'warn', 'bad', 'info', 'soft'] as const

// The living inventory of the design system. The rule it exists to enforce:
// screens take their primitives from here and only from here — a control that
// is not on this screen does not go on any other.
export function Styleguide() {
  const { t } = useTranslation()
  const [selectValue, setSelectValue] = useState('')
  const [date, setDate] = useState('')

  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <header>
        <h2 className="text-lg font-semibold">{t('styleguide.title')}</h2>
        <p className="mt-1 text-sm text-dim">{t('styleguide.rule')}</p>
      </header>

      <section className="flex flex-col gap-3">
        <SectionLabel>{t('styleguide.colors')}</SectionLabel>
        <div className="grid grid-cols-4 gap-2 md:grid-cols-5">
          {TOKENS.map((token) => (
            <Panel key={token} className="flex flex-col gap-1.5 rounded-xl p-2">
              <span
                className="h-9 rounded-lg border border-line"
                style={{ backgroundColor: `var(--${token})` }}
              />
              <code className="font-mono text-[11px] text-dim">{token}</code>
            </Panel>
          ))}
        </div>
      </section>

      <section className="flex flex-col gap-3">
        <SectionLabel>{t('styleguide.typography')}</SectionLabel>
        <Panel className="flex flex-col gap-2 p-4">
          <p className="text-lg font-semibold">{t('styleguide.sampleHeading')}</p>
          <p className="text-sm">{t('styleguide.sampleBody')}</p>
          <p className="text-sm text-dim">{t('styleguide.sampleDim')}</p>
          <p className="text-xs text-faint">{t('styleguide.sampleFaint')}</p>
          <p className="font-mono text-sm tabular-nums">2026-08-13 · 87.5 · v0.10.0</p>
        </Panel>
      </section>

      <section className="flex flex-col gap-3">
        <SectionLabel>{t('styleguide.buttons')}</SectionLabel>
        <Panel className="flex flex-wrap items-center gap-3 p-4">
          <Button variant="primary">
            <Plus aria-hidden className="size-4" />
            {t('styleguide.primary')}
          </Button>
          <Button variant="ghost">{t('styleguide.ghost')}</Button>
          <Button variant="soft">{t('styleguide.soft')}</Button>
          <Button variant="danger">
            <Trash2 aria-hidden className="size-4" />
            {t('styleguide.danger')}
          </Button>
          <Button variant="icon" size="iconMd" title={t('styleguide.iconButton')}>
            <Pencil aria-hidden />
          </Button>
          <Button variant="primary" size="sm">
            {t('styleguide.small')}
          </Button>
          <Button variant="ghost" disabled>
            {t('styleguide.disabled')}
          </Button>
        </Panel>
      </section>

      <section className="flex flex-col gap-3">
        <SectionLabel>{t('styleguide.badges')}</SectionLabel>
        <Panel className="flex flex-wrap items-center gap-3 p-4">
          {BADGES.map((variant) => (
            <Badge key={variant} variant={variant}>
              {variant}
            </Badge>
          ))}
        </Panel>
      </section>

      <section className="flex flex-col gap-3">
        <SectionLabel>{t('styleguide.forms')}</SectionLabel>
        <Panel className="grid max-w-xl gap-4 p-4">
          <Field label={t('styleguide.input')}>
            <Input placeholder={t('styleguide.placeholder')} />
          </Field>
          <Field label={t('styleguide.select')}>
            <Select
              value={selectValue}
              onChange={setSelectValue}
              placeholder={t('styleguide.placeholder')}
              options={[
                { value: 'one', label: t('styleguide.optionOne') },
                { value: 'two', label: t('styleguide.optionTwo') },
              ]}
            />
          </Field>
          <Field label={t('styleguide.date')}>
            <DatePicker value={date} onChange={setDate} placeholder={t('styleguide.placeholder')} />
          </Field>
          <Field label={t('styleguide.textarea')} hint={t('styleguide.hint')}>
            <Textarea rows={3} placeholder={t('styleguide.placeholder')} />
          </Field>
        </Panel>
      </section>

      <section className="flex flex-col gap-3">
        <SectionLabel>{t('styleguide.icons')}</SectionLabel>
        <Panel className="flex flex-wrap items-center gap-4 p-4 text-dim">
          <Star aria-hidden className="size-4" />
          <X aria-hidden className="size-4" />
          <Undo2 aria-hidden className="size-4" />
          <Plus aria-hidden className="size-4" />
          <Pencil aria-hidden className="size-4" />
          <Trash2 aria-hidden className="size-4" />
          <p className="text-xs text-faint">{t('styleguide.iconsNote')}</p>
        </Panel>
      </section>
    </div>
  )
}
