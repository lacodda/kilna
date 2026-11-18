import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { EntryStanding, PromptTemplate } from '@/lib/api/types'
import { writeTextFile } from '@/lib/api/data'
import { readSkillSet } from '@/lib/api/workspace'
import { scopeOf, type ActionScope } from '@/lib/actions'
import { queries } from '@/lib/query/queries'
import {
  answers,
  copyKey,
  grouped,
  laidOver,
  modeOf,
  NO_FILTER,
  skillName,
  skillSet,
  type Origin,
  type SkillFilter,
} from '@/lib/skills'
import { say } from '@/lib/toast'
import { allOf, say as sayLabel, useProfile } from '@/lib/useProfile'
import { PromptDialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { StatusDot } from '@/components/ui/status-dot'
import { ShippedOffers } from '@/features/settings/ShippedOffers'
import { SkillEditor } from '@/features/settings/SkillEditor'
import { useProfileDraft } from '@/features/settings/useProfileDraft'
import { useProfileProblems, useSaidProblems } from '@/features/settings/useProfileProblems'

/** A key as the profile takes one (the same shape `KeyedList` asks for). */
const KEY_SHAPE = /^[a-z][a-z0-9_-]*$/

const SKILL_FILES = [{ name: 'kilna skills', extensions: ['json'] }]

// The filter and the open skill, kept while the window lives: leaving for a
// card to try a button and coming back finds the list as it was.
let lastFilter: SkillFilter = NO_FILTER
let lastOpen: string | null = null

/**
 * Settings → Skills (FR-17): every action of the profile - a button with its
 * message, or a playbook an agent reads by its method - found by its words,
 * narrowed by kind, by what it is about, by whether it is a button and by
 * where it stands against what kilna ships, and edited whole.
 *
 * It edits the profile's draft, as the profile section does, and the same
 * bar at the foot of the pane saves or discards it. What kilna ships now
 * beside a skill its owner changed is shown at the head, to keep or take.
 */
export function SkillsSection() {
  const { t, i18n } = useTranslation()
  const profile = useProfile()
  const { config, edit, dirty } = useProfileDraft()
  const problems = useProfileProblems(config, dirty)
  const problemsAt = useSaidProblems(problems)
  const standing = useQuery(queries.shippedStanding(profile.id, 'prompts')).data ?? []
  const [filter, setFilterState] = useState<SkillFilter>(lastFilter)
  const [openKey, setOpenState] = useState<string | null>(lastOpen)
  const [adding, setAdding] = useState(false)
  const [busy, setBusy] = useState(false)

  const setFilter = (changes: Partial<SkillFilter>) =>
    setFilterState((current) => ({ ...current, ...changes }))
  const setOpen = setOpenState
  // Kept for the next visit once settled, not while drawing.
  useEffect(() => {
    lastFilter = filter
    lastOpen = openKey
  }, [filter, openKey])

  const actions = config.prompts
  const byKey = new Map<string, EntryStanding>(standing.map((one) => [one.key, one]))
  const originOf = (action: PromptTemplate): Origin => byKey.get(action.key)?.standing ?? 'own'
  const shown = actions.filter((action) => answers(action, filter, originOf(action)))
  const language = i18n.resolvedLanguage ?? 'en'

  const setActions = (next: PromptTemplate[]) => edit((current) => ({ ...current, prompts: next }))
  const setOne = (key: string, changes: Partial<PromptTemplate>) =>
    setActions(actions.map((action) => (action.key === key ? { ...action, ...changes } : action)))
  const move = (key: string, by: -1 | 1) => {
    const at = actions.findIndex((action) => action.key === key)
    const to = at + by
    if (at < 0 || to < 0 || to >= actions.length) return
    const next = [...actions]
    const [moved] = next.splice(at, 1)
    next.splice(to, 0, moved!)
    setActions(next)
  }

  const add = (key: string) => {
    if (!KEY_SHAPE.test(key)) {
      say.warn(t('skills.keyShape'))
      return
    }
    if (actions.some((action) => action.key === key)) {
      say.warn(t('skills.keyTaken', { key }))
      return
    }
    setActions([...actions, { key, label: key, template: '' }])
    setFilter(NO_FILTER)
    setOpen(key)
  }

  const run = async (task: () => Promise<void>) => {
    setBusy(true)
    try {
      await task()
    } catch (cause) {
      say.failed(cause)
    } finally {
      setBusy(false)
    }
  }
  // What goes out is the profile as it is saved, as with the whole profile:
  // a file holding edits nobody saved would be a version nobody kept.
  const doExport = () =>
    run(async () => {
      const destination = await save({
        defaultPath: 'skills.json',
        title: t('skills.exportTitle'),
        filters: SKILL_FILES,
      })
      if (destination === null) return
      const written = await writeTextFile(destination, skillSet(profile.config.prompts))
      say.ok(t('skills.exported', { count: profile.config.prompts.length, path: written }))
    })
  const doImport = () =>
    run(async () => {
      const source = await open({ title: t('skills.importTitle'), filters: SKILL_FILES })
      if (typeof source !== 'string') return
      const incoming = await readSkillSet(source)
      const merged = laidOver(actions, incoming)
      setActions(merged.actions)
      say.ok(t('skills.imported', { replaced: merged.replaced, added: merged.added }))
    })

  const kindOptions = config.work_kinds.map((kind) => ({
    value: kind.key,
    label: sayLabel(kind.label),
  }))
  const scopes: ActionScope[] = [
    'work',
    'scene',
    'selection',
    'style',
    'comment',
    'canon',
    'release',
    'cover',
    'phrases',
    'lab',
    'board',
  ]
  const origins: Origin[] = ['standard', 'changed', 'update', 'own']

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <ShippedOffers only="prompts" />

      <FieldGroup label={t('skills.title')} help={t('skills.hint')}>
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2">
            <Input
              className="w-64"
              type="search"
              value={filter.query}
              placeholder={t('skills.search')}
              aria-label={t('skills.search')}
              onChange={(event) => setFilter({ query: event.target.value })}
            />
            <Select
              className="w-40"
              aria-label={t('skills.filterKind')}
              placeholder={t('skills.anyKind')}
              value={filter.kind}
              onChange={(kind) => setFilter({ kind })}
              options={kindOptions}
            />
            <Select
              className="w-44"
              aria-label={t('skills.filterScope')}
              placeholder={t('skills.anyScope')}
              value={filter.scope}
              onChange={(scope) => setFilter({ scope: scope as ActionScope | '' })}
              options={scopes.map((scope) => ({ value: scope, label: t(`skills.scope.${scope}`) }))}
            />
            <Select
              className="w-44"
              aria-label={t('skills.filterOrigin')}
              placeholder={t('skills.anyOrigin')}
              value={filter.origin}
              onChange={(origin) => setFilter({ origin: origin as Origin | '' })}
              options={origins.map((origin) => ({
                value: origin,
                label: t(`skills.origin.${origin}`),
              }))}
            />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <SegmentedControl
              aria-label={t('skills.filterMode')}
              value={filter.mode === '' ? 'all' : filter.mode}
              onValueChange={(mode) =>
                setFilter({ mode: mode === 'all' ? '' : (mode as SkillFilter['mode']) })
              }
            >
              <Segment value="all">{t('skills.mode.all')}</Segment>
              <Segment value="button">{t('skills.mode.button')}</Segment>
              <Segment value="playbook">{t('skills.mode.playbook')}</Segment>
            </SegmentedControl>
            <span className="flex-1" />
            <Button size="sm" onClick={() => setAdding(true)}>
              {t('skills.add')}
            </Button>
            <Button size="sm" disabled={busy} onClick={doExport}>
              {t('skills.export')}
            </Button>
            <Button size="sm" disabled={busy} onClick={doImport}>
              {t('skills.import')}
            </Button>
          </div>
        </div>
      </FieldGroup>

      <p className="text-xs text-dim">
        {t('skills.count', { shown: shown.length, count: actions.length })}
      </p>

      {grouped(shown).map((group) => (
        <section
          key={group.category ?? 'none'}
          aria-label={t(`skills.category.${group.category ?? 'none'}`)}
          className="flex flex-col"
        >
          <h3 className="caption pb-1">{t(`skills.category.${group.category ?? 'none'}`)}</h3>
          <div className="flex flex-col rounded-md border border-line">
            {group.actions.map((action) => {
              const origin = originOf(action)
              const wrong = problemsAt(['action', action.key])
              const isOpen = openKey === action.key
              const standard = byKey.get(action.key)?.standard as PromptTemplate | undefined
              return (
                <div key={action.key} className="border-b border-line last:border-b-0">
                  <RowButton
                    className="border-b-0"
                    aria-expanded={isOpen}
                    onClick={() => setOpen(isOpen ? null : action.key)}
                    start={
                      <span className="w-36 truncate font-mono text-xs text-faint">
                        {action.key}
                      </span>
                    }
                    description={
                      action.when == null ? undefined : sayLabel(action.when) || undefined
                    }
                    end={
                      <span className="flex items-center gap-1.5">
                        {wrong.length > 0 && (
                          <StatusDot status="bad" size="sm" label={t('editor.partWrong')} />
                        )}
                        <Badge variant={modeOf(action) === 'button' ? 'outline' : 'info'}>
                          {t(`skills.mode.${modeOf(action)}`)}
                        </Badge>
                        <Badge variant={origin === 'update' ? 'accent' : 'soft'}>
                          {t(`skills.origin.${origin}`)}
                        </Badge>
                        <span className="w-24 truncate text-xs text-faint">
                          {(action.kinds ?? []).length === 0
                            ? t('editor.actionKindsAll')
                            : action
                                .kinds!.map((key) =>
                                  sayLabel(
                                    config.work_kinds.find((kind) => kind.key === key)?.label ??
                                      key,
                                  ),
                                )
                                .join(', ')}
                        </span>
                      </span>
                    }
                  >
                    {skillName(action, language)}
                    {scopeOf(action) !== 'work' && (
                      <span className="ml-2 text-xs text-faint">
                        {t(`skills.scope.${scopeOf(action)}`)}
                      </span>
                    )}
                  </RowButton>
                  {isOpen && (
                    <div className="flex flex-col gap-3 border-t border-line p-3">
                      <SkillEditor
                        action={action}
                        kinds={config.work_kinds}
                        roles={allOf(config, 'version_roles')}
                        boards={config.boards ?? []}
                        set={(changes) => setOne(action.key, changes)}
                        problems={wrong}
                      />
                      <div className="flex flex-wrap gap-2">
                        {standard !== undefined && origin !== 'standard' && (
                          <Button
                            size="sm"
                            onClick={() =>
                              setActions(
                                actions.map((one) => (one.key === action.key ? standard : one)),
                              )
                            }
                          >
                            {t('skills.reset')}
                          </Button>
                        )}
                        <Button
                          size="sm"
                          onClick={() => {
                            const key = copyKey(actions, action.key)
                            const at = actions.findIndex((one) => one.key === action.key)
                            const next = [...actions]
                            next.splice(at + 1, 0, { ...action, key })
                            setActions(next)
                            setOpen(key)
                          }}
                        >
                          {t('skills.duplicate')}
                        </Button>
                        <Button size="sm" onClick={() => move(action.key, -1)}>
                          {t('skills.earlier')}
                        </Button>
                        <Button size="sm" onClick={() => move(action.key, 1)}>
                          {t('skills.later')}
                        </Button>
                        <span className="flex-1" />
                        <Button
                          size="sm"
                          variant="danger"
                          onClick={() => {
                            setActions(actions.filter((one) => one.key !== action.key))
                            setOpen(null)
                          }}
                        >
                          {t('editor.removeAction')}
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        </section>
      ))}

      <PromptDialog
        open={adding}
        onOpenChange={setAdding}
        title={t('skills.addTitle')}
        description={t('skills.addDescription')}
        label={t('skills.key')}
        confirmLabel={t('skills.add')}
        onSubmit={(key) => {
          setAdding(false)
          add(key.trim())
        }}
      />
    </div>
  )
}
