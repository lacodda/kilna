import type { ReactNode } from 'react'
import { QueryClientProvider, type QueryClient } from '@tanstack/react-query'
import { LocaleProvider } from 'dowel-ui'
import { Toaster } from '@/app/Toaster'
import { useLanguage } from '@/lib/language'
import { RunEventsBridge } from '@/lib/runEvents'

/*
 * The interface's language, handed to every dowel component that formats a
 * date, a month or a number - not the machine's. A Russian interface on an
 * en-US machine once drew the date picker's weeks from Sunday while the big
 * month grid beside it drew them from Monday, and two calendars on one screen
 * disagreeing about the week reads as a bug in the data. Left alone the
 * components read `<html lang>`, which follows the interface too, but only as
 * a value read during some render; the provider re-renders with the language,
 * so a month already on screen changes with the rest of the window.
 */
function AppLocale({ children }: { children: ReactNode }) {
  const { resolved } = useLanguage()
  return <LocaleProvider locale={resolved}>{children}</LocaleProvider>
}

/**
 * Everything the app is rendered inside, apart from the router.
 *
 * One component rather than a nesting written out in `main.tsx`, because the
 * tests render the app too: a provider added here reaches both, and a test
 * cannot pass against a tree the window does not have. The router is left to
 * the caller - the window follows its address bar, a test starts at a path.
 */
export function Providers({ client, children }: { client: QueryClient; children: ReactNode }) {
  return (
    <QueryClientProvider client={client}>
      {/* The assistant's runs are heard once, here, for every screen. */}
      <RunEventsBridge>
        <AppLocale>
          {/* Wraps the app rather than sitting beside it: a toast raised from
              anywhere inside has to reach the same provider. */}
          <Toaster>{children}</Toaster>
        </AppLocale>
      </RunEventsBridge>
    </QueryClientProvider>
  )
}
