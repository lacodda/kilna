import { useLocation } from 'react-router'
import { useTranslation } from 'react-i18next'

interface Props {
  works: number
}

// Maps the first path segment to the nav key that names the screen.
function screenKey(pathname: string): string {
  const segment = pathname.split('/')[1]
  switch (segment) {
    case 'catalogue':
      return 'nav.catalogue'
    case 'calendar':
      return 'nav.calendar'
    case 'trash':
      return 'nav.trash'
    case 'settings':
      return 'nav.data'
    case 'styleguide':
      return 'nav.styleguide'
    default:
      return 'nav.works'
  }
}

export function Topbar({ works }: Props) {
  const { t } = useTranslation()
  const location = useLocation()

  return (
    <header className="flex items-center gap-2.5 border-b border-line px-4">
      <span className="text-[13px] font-semibold">{t(screenKey(location.pathname))}</span>
      <p className="ml-auto text-xs text-faint">
        {t('status.works')} {works}
      </p>
    </header>
  )
}
