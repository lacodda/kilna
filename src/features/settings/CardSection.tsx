import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import type { OverviewLayout, ProfileConfig, Workspace } from '@/lib/api/types'
import { updateProfileConfig } from '@/lib/api/workspace'
import { LAYOUTS, boardOf } from '@/lib/overview'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { useCardView } from '@/features/work/cardView'
import { DEFAULT_TAB, openingTab, tabsOfKind, type Tab } from '@/features/work/tabs'
import { FieldGroup } from '@/components/ui/field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Select } from '@/components/AppSelect'
import { Switch } from '@/components/ui/switch'

/**
 * What the work card draws and where it opens.
 *
 * What stands beside the title is this machine's, for the reason the theme
 * is: it answers "what do I want to look at", not "what does this craft
 * consist of". Where a card opens (v0.90.1) and how the overview is laid out
 * (v0.82) are the profile's, for the catalogue columns' reason - a craft reads
 * its works through its own board, and a song is opened for its text where a
 * short is opened for its board - and each is written the moment it is
 * chosen, as the columns are.
 * There is no Save button here either way: the card two routes away changes the
 * moment a control moves. The profile's editor, which has one, says so in a
 * bar of its own - and its draft follows this write rather than undoing it
 * (`lib/profileDraft`).
 */
export function CardSection() {
  const { t } = useTranslation()
  const { view, setCardView } = useCardView()
  const profile = useProfile()
  const client = useQueryClient()

  const keepLayout = useAppMutation({
    // The layout alone changes; the placements stay as they are. An empty
    // list means the placement kilna ships, so choosing a layout on a profile
    // that never chose one invents nothing.
    mutationFn: (layout: OverviewLayout) =>
      updateProfileConfig(profile.id, {
        ...profile.config,
        overview: { layout, widgets: profile.config.overview?.widgets ?? [] },
      }),
    failure: 'toast.profileSaveFailed',
    refresh: refresh.profile,
    onSuccess: (saved) => {
      // The saved profile in place now rather than when the refetch lands, so
      // the segment does not flick back to the old layout for a round trip.
      client.setQueryData<Workspace>(keys.workspace, (current) =>
        current?.profile?.id === saved.id ? { ...current, profile: saved } : current,
      )
    },
  })
  const keepOpening = useAppMutation({
    // Made over the profile in the cache and written back there before it
    // goes, so a second kind chosen while the first is on its way is made
    // over the first rather than over the profile as it was. A refusal
    // reads the profile again (`refresh.profile`). The overview is written
    // as nothing: absent means it, and a kind that never chose keeps
    // following the default if it moves.
    mutationFn: ({ kind, tab }: { kind: string; tab: Tab }) => {
      const held = client.getQueryData<Workspace>(keys.workspace)?.profile
      const config: ProfileConfig = held?.id === profile.id ? held.config : profile.config
      const next: ProfileConfig = {
        ...config,
        work_kinds: config.work_kinds.map((entry) =>
          entry.key === kind ? { ...entry, open_on: tab === DEFAULT_TAB ? null : tab } : entry,
        ),
      }
      client.setQueryData<Workspace>(keys.workspace, (current) =>
        current?.profile?.id === profile.id
          ? { ...current, profile: { ...current.profile, config: next } }
          : current,
      )
      return updateProfileConfig(profile.id, next)
    },
    failure: 'toast.profileSaveFailed',
    refresh: refresh.profile,
  })

  // What was just pressed, while it is on its way; a refusal falls back to
  // the stored layout by itself.
  const layout =
    keepLayout.isPending && keepLayout.variables !== undefined
      ? keepLayout.variables
      : boardOf(profile.config).layout

  return (
    // Each field says what it does under itself, as the mockup's fields do,
    // rather than under a heading above it: the explanation is read after the
    // control it explains, and the control is not pushed down the pane by it.
    <div className="flex flex-col gap-4">
      {/* A row per kind: a song is opened for its text, an audio release for
          where it stands, a short for its board. Each kind is offered the
          tabs every work of it draws. */}
      <FieldGroup
        className="max-w-105"
        label={t('settings.defaultTabLabel')}
        help={t('settings.defaultTabHint')}
      >
        <div className="grid grid-cols-[minmax(0,9rem)_minmax(0,1fr)] items-center gap-x-3 gap-y-1.5">
          {profile.config.work_kinds.map((kind) => {
            const name = sayLabel(kind.label)
            return (
              <Fragment key={kind.key}>
                <span className="truncate text-sm text-dim">{name}</span>
                <Select
                  aria-label={t('settings.defaultTabFor', { kind: name })}
                  value={openingTab(profile.config, kind.key)}
                  onChange={(next) => keepOpening.mutate({ kind: kind.key, tab: next as Tab })}
                  options={tabsOfKind(profile.config, kind.key).map((tab) => ({
                    value: tab,
                    label: t(`card.tab.${tab}`),
                  }))}
                />
              </Fragment>
            )
          })}
        </div>
      </FieldGroup>

      {/* Five options, every one worth seeing at once: a segment, not a menu.
          A little wider than the fields beside it - five names in a row are
          wider than one select in Russian, and a segment that wraps is no
          longer one row. */}
      <FieldGroup
        className="max-w-120"
        label={t('settings.overviewLayoutLabel')}
        help={t('settings.overviewLayoutHint')}
      >
        <SegmentedControl
          aria-label={t('settings.overviewLayoutLabel')}
          value={layout}
          onValueChange={(next) => keepLayout.mutate(next as OverviewLayout)}
        >
          {LAYOUTS.map((entry) => (
            <Segment key={entry} value={entry}>
              {t(`settings.overviewLayout.${entry}`)}
            </Segment>
          ))}
        </SegmentedControl>
      </FieldGroup>

      <FieldGroup className="max-w-105" label={t('data.cardView')} help={t('data.cardViewHint')}>
        <Switch checked={view.metaStrip} onCheckedChange={(on) => setCardView({ metaStrip: on })}>
          {t('data.showMetaStrip')}
        </Switch>
      </FieldGroup>
    </div>
  )
}
