import type { ReactNode } from 'react'
import { MemoryRouter } from 'react-router'
import { render, renderHook, waitFor } from '@testing-library/react'
import type { QueryClient } from '@tanstack/react-query'
import App from '@/App'
import { Providers } from '@/app/Providers'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import type { Profile } from '@/lib/api'
import { createQueryClient } from '@/lib/query'
import { ProfileContext } from '@/lib/useProfile'

/**
 * The whole window at `path`: the same providers and boundary as `main.tsx`,
 * a router that starts where the test says, and a query cache of its own.
 */
export function renderApp(path: string) {
  const client = createQueryClient()
  const view = render(
    <Providers client={client}>
      <MemoryRouter initialEntries={[path]}>
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </MemoryRouter>
    </Providers>,
  )
  return { ...view, client }
}

/**
 * One component inside the app's providers and a profile, without the shell -
 * for a hook or a panel whose question is its own behaviour.
 */
export function renderWithin(
  ui: ReactNode,
  { profile, path = '/' }: { profile: Profile; path?: string },
) {
  const client = createQueryClient()
  const view = render(
    <Providers client={client}>
      <MemoryRouter initialEntries={[path]}>
        <ProfileContext value={profile}>{ui}</ProfileContext>
      </MemoryRouter>
    </Providers>,
  )
  return { ...view, client }
}

/** Until nothing is being fetched or written: the screen has said what it will say. */
export async function settled(client: QueryClient): Promise<void> {
  await waitFor(() => {
    if (client.isFetching() + client.isMutating() > 0) throw new Error('still loading')
  })
}

/** A hook inside the app's providers: the query cache, the language, toasts. */
export function renderHookWithin<Result, Props>(
  hook: (props: Props) => Result,
  initialProps?: Props,
) {
  const client = createQueryClient()
  const view = renderHook(hook, {
    initialProps,
    wrapper: ({ children }) => <Providers client={client}>{children}</Providers>,
  })
  return { ...view, client }
}
