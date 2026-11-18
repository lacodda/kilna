import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { assistantStatus } from '@/lib/api/assistant'
import { seedDemo, welcomeSpace } from '@/lib/api/spaces'
import { createWork } from '@/lib/api/works'
import { getWorkspace } from '@/lib/api/workspace'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { StatusDot } from '@/components/ui/status-dot'
import { kindLook, SPACE_KINDS } from '@/features/spaces/look'
import { LookPicker } from '@/features/spaces/LookPicker'

type Step = 'check' | 'space' | 'first'

/**
 * The first start of a new machine (v0.129, FR-16): kilna made a space so
 * the window has somewhere to stand, and this welcomes the person into it.
 * Whether Claude Code is here - the assistant waits without it, nothing else
 * does; what the space is made of, its name and look, and a demo to look
 * around in; then the first work, by hand. Put off with Later, it comes back
 * at the next start; answered, never again.
 */
export function WelcomeDialog() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const current = useQuery(queries.currentSpace())
  const space = current.data?.space
  const [later, setLater] = useState(false)
  const [step, setStep] = useState<Step>('check')
  const [done, setDone] = useState(false)

  const open = (space?.welcome === true || step === 'first') && !later && !done
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setLater(true)
      }}
      title={t(`welcome.${step}.title`)}
      description={t(`welcome.${step}.description`)}
      size="lg"
      dirty={step !== 'check'}
      // Each step's answer at the foot, where every dialog keeps it; the
      // forms are submitted from there by their ids.
      footer={
        step === 'check' ? (
          <Button variant="primary" onClick={() => setStep('space')}>
            {t('welcome.next')}
          </Button>
        ) : step === 'space' ? (
          <Button variant="primary" type="submit" form="welcome-space">
            {t('welcome.space.make')}
          </Button>
        ) : (
          <>
            <Button onClick={() => setDone(true)}>{t('welcome.first.skip')}</Button>
            <Button variant="primary" type="submit" form="welcome-first">
              {t('welcome.first.make')}
            </Button>
          </>
        )
      }
    >
      {step === 'check' && <Check />}
      {step === 'space' && (
        <Shape
          kind={space?.kind ?? 'music'}
          onShaped={async (demo) => {
            navigate('/dashboard')
            await client.resetQueries()
            if (demo) {
              setDone(true)
              return
            }
            setStep('first')
          }}
        />
      )}
      {step === 'first' && (
        <First
          onMade={(id) => {
            setDone(true)
            navigate(`/works/${id}`)
          }}
        />
      )}
    </Dialog>
  )
}

/** Whether Claude Code is on this machine: the assistant's runs are it. */
function Check() {
  const { t } = useTranslation()
  const status = useQuery({ queryKey: ['welcome', 'assistant'], queryFn: assistantStatus })
  const found = status.data?.available === true
  return (
    <div className="flex flex-col gap-3">
      {status.data === undefined ? (
        <p className="text-sm text-dim">{t('welcome.check.looking')}</p>
      ) : (
        <p className="flex items-center gap-2 text-sm">
          <StatusDot
            status={found ? 'good' : 'warn'}
            size="sm"
            label={found ? t('welcome.check.found') : t('welcome.check.missing')}
          />
          {found
            ? t('welcome.check.foundVersion', { version: status.data.version ?? '' })
            : t('welcome.check.missing')}
        </p>
      )}
      {!found && status.data !== undefined && (
        <p className="text-xs text-dim">{t('welcome.check.install')}</p>
      )}
    </div>
  )
}

/** What the space is made of, its name and look, and a demo or not. */
function Shape({
  kind: made,
  onShaped,
}: {
  kind: string
  onShaped: (demo: boolean) => Promise<void>
}) {
  const { t, i18n } = useTranslation()
  const types = useQuery(queries.spaceTypes()).data ?? []
  const [madeOf, setMadeOf] = useState<string>(made)
  const type = types.find((entry) => `type:${entry.id}` === madeOf)
  const kind = type?.kind ?? (madeOf.startsWith('type:') ? 'music' : madeOf)
  const [name, setName] = useState<string | null>(null)
  const shownName = name ?? type?.name ?? t(`spaces.kinds.${kind}`)
  const [look, setLook] = useState<{ colour: string | null; mark: string | null }>({
    colour: null,
    mark: null,
  })
  const [demo, setDemo] = useState(false)

  const shape = useAppMutation({
    mutationFn: async () => {
      await welcomeSpace({
        name: shownName.trim(),
        kind,
        colour: look.colour ?? kindLook(kind).colour,
        mark: look.mark ?? kindLook(kind).mark,
        ...(type === undefined ? {} : { type_id: type.id }),
      })
      if (demo) {
        const report = await seedDemo(i18n.resolvedLanguage ?? 'en')
        say.ok(t('welcome.demoAdded', { count: report.works }))
      }
    },
    failure: 'welcome.failed',
    onSuccess: () => void onShaped(demo),
  })

  return (
    <form
      id="welcome-space"
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (shownName.trim() !== '' && !shape.isPending) shape.mutate()
      }}
    >
      <FieldGroup label={t('spaces.kind')} help={type?.description ?? t(`spaces.kindHint.${kind}`)}>
        <ChipGroup
          aria-label={t('spaces.kind')}
          value={[madeOf]}
          onValueChange={(next) => {
            const picked = next.find((entry) => entry !== madeOf)
            if (picked !== undefined) setMadeOf(picked)
          }}
        >
          {SPACE_KINDS.map((entry) => (
            <Chip key={entry} value={entry}>
              {t(`spaces.kinds.${entry}`)}
            </Chip>
          ))}
          {types.map((entry) => (
            <Chip key={entry.id} value={`type:${entry.id}`}>
              {entry.name}
            </Chip>
          ))}
        </ChipGroup>
      </FieldGroup>
      <Field label={t('spaces.name')}>
        <Input value={shownName} onChange={(event) => setName(event.target.value)} maxLength={80} />
      </Field>
      <LookPicker
        colour={look.colour ?? kindLook(kind).colour}
        mark={look.mark ?? kindLook(kind).mark}
        onChange={(next) => setLook({ ...look, ...next })}
      />
      <Checkbox checked={demo} onCheckedChange={(next) => setDemo(next === true)}>
        {t('welcome.space.demo')}
      </Checkbox>
      <p className="-mt-2 text-xs text-dim">{t('welcome.space.demoHint')}</p>
    </form>
  )
}

/** The first work, by hand: a title, in the space's first kind. */
function First({ onMade }: { onMade: (id: string) => void }) {
  const { t } = useTranslation()
  const [title, setTitle] = useState('')
  const make = useAppMutation({
    mutationFn: async () => {
      const workspace = await getWorkspace()
      const kind = workspace.profile?.config.work_kinds[0]?.key ?? 'song'
      return createWork({ kind, title: title.trim() })
    },
    failure: 'welcome.failed',
    onSuccess: (work) => onMade(work.id),
  })
  return (
    <form
      id="welcome-first"
      className="flex flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault()
        if (title.trim() !== '' && !make.isPending) make.mutate()
      }}
    >
      <Field label={t('welcome.first.titleLabel')}>
        <Input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder={t('welcome.first.placeholder')}
          maxLength={200}
        />
      </Field>
    </form>
  )
}
