import { Component, type ErrorInfo, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { RotateCcw, TriangleAlert } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { logWindowError } from '@/lib/api/data'
import { openExternal } from '@/lib/link'

interface Props {
  children: ReactNode
  /** Changing this resets the boundary — used to clear the error on navigation. */
  resetKey?: string
}

interface State {
  error: Error | null
}

/** The project's issues, a new one written for this crash (v0.162): its
 *  message and the top of its stack, with any path under a home cut to `~`. */
export function issueFor(error: Error): string {
  const home = /([A-Za-z]:)?[\\/](Users|home)[\\/][^\\/\s]+/g
  const clean = (text: string) => text.replace(home, '~')
  const stack = clean((error.stack ?? '').split('\n').slice(0, 6).join('\n'))
  const body = [
    '**What happened**',
    '',
    '<!-- What were you doing when the screen stopped? -->',
    '',
    '**The error**',
    '',
    '```',
    clean(error.message),
    stack,
    '```',
    '',
    '_Settings → About kilna → Copy the diagnostics, and paste them here._',
  ].join('\n')
  const query = new URLSearchParams({
    title: `A screen stopped working: ${clean(error.message).slice(0, 80)}`,
    body,
  })
  return `https://github.com/lacodda/kilna/issues/new?${query.toString()}`
}

// The recovery panel, split out so it can use hooks the class component cannot.
function Fallback({ error, onRetry }: { error: Error; onRetry: () => void }) {
  const { t } = useTranslation()

  return (
    <div className="flex h-full items-center justify-center p-8">
      <Panel className="flex max-w-lg flex-col items-start gap-4 p-6">
        <div className="flex items-center gap-2 text-bad">
          <TriangleAlert aria-hidden className="size-5" />
          <h2 className="font-semibold">{t('crash.title')}</h2>
        </div>

        <p className="text-sm text-dim">{t('crash.body')}</p>

        {/* The message is for whoever files the bug, so it stays verbatim —
            but folded away, because it is not an instruction to the reader. */}
        <details className="w-full">
          <summary className="cursor-pointer text-xs text-faint">{t('crash.details')}</summary>
          <pre className="selectable mt-2 max-h-40 overflow-auto rounded-md bg-softer p-3 font-mono text-xs text-dim">
            {error.message}
          </pre>
        </details>

        <div className="flex flex-wrap gap-2">
          <Button variant="primary" onClick={onRetry}>
            <RotateCcw aria-hidden />
            {t('crash.retry')}
          </Button>
          {/* An issue already written: the message and where, nothing of the
              person's own - they read it before they send it (v0.162). */}
          <Button variant="ghost" onClick={() => void openExternal(issueFor(error))}>
            {t('crash.report')}
          </Button>
        </div>
      </Panel>
    </div>
  )
}

/**
 * Stops one broken component from taking the window with it.
 *
 * Before this, a render error left a white screen with the message only in the
 * devtools console — which a person running the built app does not have open.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidUpdate(previous: Props) {
    // Navigating away from the screen that crashed should not keep showing its
    // wreckage — the next screen deserves a clean try.
    if (previous.resetKey !== this.props.resetKey && this.state.error !== null) {
      this.setState({ error: null })
    }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // No telemetry in kilna (that is a 1.x decision with consent attached).
    // The record stays on this machine: in the console for a development
    // build, and in the application's log file for a release one, which has
    // no console - that file is what a bug report quotes.
    console.error('component crashed', error, info.componentStack)
    // The innermost component is enough to find the screen; the whole stack
    // is in the console of a development build.
    const where = info.componentStack?.trim().split(/\r?\n/)[0]?.trim() ?? ''
    logWindowError(`a screen stopped working: ${error.message} ${where}`).catch(() => {
      // The log is a record, not a duty: a window that cannot reach it has
      // already shown the person the panel below.
    })
  }

  render() {
    if (this.state.error !== null) {
      return <Fallback error={this.state.error} onRetry={() => this.setState({ error: null })} />
    }

    return this.props.children
  }
}
