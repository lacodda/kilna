import { Component, type ErrorInfo, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { RotateCcw, TriangleAlert } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'

interface Props {
  children: ReactNode
  /** Changing this resets the boundary — used to clear the error on navigation. */
  resetKey?: string
}

interface State {
  error: Error | null
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
          <pre className="selectable mt-2 max-h-40 overflow-auto rounded-[9px] bg-softer p-3 font-mono text-xs text-dim">
            {error.message}
          </pre>
        </details>

        <Button variant="primary" onClick={onRetry}>
          <RotateCcw aria-hidden className="size-4" />
          {t('crash.retry')}
        </Button>
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
    // No telemetry in kilna (that is a 1.x decision with consent attached), so
    // the console is the only record. Keep it: it is what a bug report quotes.
    console.error('component crashed', error, info.componentStack)
  }

  render() {
    if (this.state.error !== null) {
      return <Fallback error={this.state.error} onRetry={() => this.setState({ error: null })} />
    }

    return this.props.children
  }
}
