import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Splash as SplashScreen } from '@/components/ui/splash'
import { Mark } from '@/components/shell/Mark'

/**
 * What the window shows while the workspace is being opened.
 *
 * The same picture `index.html` painted before the bundle arrived - the mark,
 * the name, the promise, the version - so that nothing jumps when React takes
 * over, plus the two lines only the application can say: what it is doing, in
 * the person's language, and one thing worth knowing. The static one is taken
 * down here, on the first render, because from this moment the page is the
 * one drawing.
 */
export function Splash() {
  const { t } = useTranslation()

  useEffect(() => {
    document.getElementById('splash')?.remove()
  }, [])

  return (
    <SplashScreen
      mark={<Mark className="size-14" />}
      name={t('app.name')}
      tagline={t('app.tagline')}
      version={`v${__APP_VERSION__}`}
      status={t('splash.opening')}
      tip={t('splash.tip')}
    />
  )
}
