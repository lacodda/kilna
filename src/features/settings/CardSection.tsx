import { useTranslation } from 'react-i18next'
import { useCardView } from '@/features/work/cardView'
import { DEFAULT_TAB_CHOICES } from '@/features/work/tabs'
import { FieldGroup } from '@/components/ui/field'
import { Select } from '@/components/AppSelect'
import { Switch } from '@/components/ui/switch'

/**
 * What the work card draws and where it opens, as this machine likes it.
 *
 * Machine settings rather than profile ones, for the reason the theme is: they
 * answer "what do I want to look at", not "what does this craft consist of".
 * There is no Save button here - the card two routes away changes the moment a
 * control moves. The profile, which has one, says so in a bar of its own.
 */
export function CardSection() {
  const { t } = useTranslation()
  const { view, setCardView } = useCardView()

  return (
    // Each field says what it does under itself, as the mockup's fields do,
    // rather than under a heading above it: the explanation is read after the
    // control it explains, and the control is not pushed down the pane by it.
    <div className="flex max-w-105 flex-col gap-4">
      <FieldGroup label={t('settings.defaultTabLabel')} help={t('settings.defaultTabHint')}>
        <Select
          aria-label={t('settings.defaultTabLabel')}
          value={view.defaultTab}
          onChange={(next) => setCardView({ defaultTab: next as typeof view.defaultTab })}
          options={DEFAULT_TAB_CHOICES.map((tab) => ({
            value: tab,
            label: t(`card.tab.${tab}`),
          }))}
        />
      </FieldGroup>

      <FieldGroup label={t('data.cardView')} help={t('data.cardViewHint')}>
        <Switch checked={view.metaStrip} onCheckedChange={(on) => setCardView({ metaStrip: on })}>
          {t('data.showMetaStrip')}
        </Switch>
      </FieldGroup>
    </div>
  )
}
