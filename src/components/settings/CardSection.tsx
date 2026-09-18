import { useTranslation } from 'react-i18next'
import { useCardView } from '@/lib/cardView'
import { DEFAULT_TAB_CHOICES } from '@/components/card/tabs'
import { Field } from '@/components/ui/Field'
import { Select } from '@/components/ui/AppSelect'
import { Switch } from '@/components/ui/switch'

/**
 * What the work card draws and where it opens, as this machine likes it.
 *
 * Machine settings rather than profile ones, for the reason the theme is: they
 * answer "what do I want to look at", not "what does this craft consist of".
 * There is no Save button here - the card two routes away changes the moment a
 * control moves.
 */
export function CardSection() {
  const { t } = useTranslation()
  const { view, setCardView } = useCardView()

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('settings.defaultTab')}</h3>
        <p className="text-sm text-dim">{t('settings.defaultTabHint')}</p>
        <Field label={t('settings.defaultTabLabel')}>
          <Select
            className="max-w-xs"
            value={view.defaultTab}
            onChange={(next) => setCardView({ defaultTab: next as typeof view.defaultTab })}
            options={DEFAULT_TAB_CHOICES.map((tab) => ({ value: tab, label: t(`card.tab.${tab}`) }))}
          />
        </Field>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.cardView')}</h3>
        <p className="text-sm text-dim">{t('data.cardViewHint')}</p>
        <Switch checked={view.metaStrip} onCheckedChange={(on) => setCardView({ metaStrip: on })}>
          {t('data.showMetaStrip')}
        </Switch>
      </section>
    </div>
  )
}
