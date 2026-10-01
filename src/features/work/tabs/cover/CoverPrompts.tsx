import { useTranslation } from 'react-i18next'
import { AlertTriangle } from 'lucide-react'
import type { Cover, CoverPrompts as Prompts, CoverView } from '@/lib/api/types'
import { formatStamp } from '@/lib/format'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { CopyButton } from '@/components/ui/copy-button'
import { Panel } from '@/components/ui/panel'
import { Switch } from '@/components/ui/switch'
import { DraftText } from '@/features/cover/Section'

type Part = 'picture' | 'negative' | 'typography'

interface Props {
  cover: Cover
  view: CoverView
  disabled?: boolean
  change: (patch: (cover: Cover) => Cover) => void
}

/**
 * The prompt, as it is copied into a generator: the picture, what to keep
 * out, and - when the title goes apart - the edit that letters it.
 *
 * Written on the Rust side from the cover, so what is copied here is what an
 * agent reading the work gets too. Under each block, the person's own words
 * for it: they close the block, after everything built - the place a cover
 * written before the constructor keeps its prompt whole.
 *
 * Copying records the prompt on the cover (`sent`): what the picture was
 * drawn from stays with it when the channel or a brick changes later, and
 * the panel says when the prompt has moved on since.
 */
export function CoverPrompts({ cover, view, disabled, change }: Props) {
  const { t } = useTranslation()
  const { prompts } = view
  const parts: Part[] =
    prompts.typography === null ? ['picture', 'negative'] : ['picture', 'negative', 'typography']
  // Own words for the lettering are always offered on a built cover: they
  // join the lettering wherever it goes.
  const own: Part[] = view.built ? ['picture', 'negative', 'typography'] : parts

  const record = (sent: Prompts) =>
    change((c) => ({
      ...c,
      sent: {
        picture: sent.picture,
        negative: sent.negative,
        typography: sent.typography,
        at: new Date().toISOString(),
      },
    }))
  const copied = (ok: boolean) => {
    if (!ok) say.failed(t('work.copyFailed'))
    else record(prompts)
  }
  const whole = [
    prompts.picture,
    prompts.negative === '' ? '' : `NEGATIVE: ${prompts.negative}`,
    prompts.typography ?? '',
  ]
    .filter((part) => part.trim() !== '')
    .join('\n\n---\n\n')

  return (
    <Panel className="flex flex-col">
      <header className="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2">
        <b className="text-sm font-semibold">{t('cover.prompt.title')}</b>
        <span className="text-2xs text-faint">{t('cover.prompt.hint')}</span>
        {view.built && (
          <Switch
            className="ml-auto"
            checked={cover.lettering.apart}
            disabled={disabled}
            onCheckedChange={(apart) =>
              change((c) => ({ ...c, lettering: { ...c.lettering, apart } }))
            }
          >
            {t('cover.prompt.apart')}
          </Switch>
        )}
      </header>

      <div className="flex flex-col gap-3 p-3">
        {!view.built && <p className="text-xs text-dim">{t('cover.prompt.byHand')}</p>}
        {parts.map((part) => {
          const text = prompts[part] ?? ''
          const name = t(`cover.prompt.part.${part}`)
          return (
            <section key={part} aria-label={name} className="group flex min-w-0 flex-col gap-1.5">
              <header className="flex min-h-5 items-center gap-2">
                <span className="caption">{name}</span>
                <span className="font-mono text-2xs text-faint">{part}</span>
                <CopyButton
                  value={text}
                  label={t('cover.prompt.copy', { part: name })}
                  copiedLabel={t('cover.copied')}
                  title={t('cover.prompt.copy', { part: name })}
                  disabled={text.trim() === ''}
                  className="ml-auto"
                  onCopy={copied}
                />
              </header>
              {view.built &&
                (text.trim() === '' ? (
                  <p className="text-xs text-faint">{t('cover.prompt.empty')}</p>
                ) : (
                  <pre className="selectable rounded-md border border-line bg-soft px-2.5 py-2 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap text-dim">
                    {text}
                  </pre>
                ))}
              {own.includes(part) && (
                <OwnWords
                  part={part}
                  value={cover[part]}
                  built={view.built}
                  disabled={disabled}
                  onCommit={(words) => change((c) => ({ ...c, [part]: words }))}
                />
              )}
            </section>
          )
        })}
        {view.built && prompts.typography === null && (
          <OwnWords
            part="typography"
            value={cover.typography}
            built
            disabled={disabled}
            onCommit={(typography) => change((c) => ({ ...c, typography }))}
          />
        )}
      </div>

      <footer className="flex flex-wrap items-center gap-2 border-t border-line px-3 py-2">
        <Button
          variant="primary"
          size="sm"
          disabled={whole === ''}
          onClick={() => {
            navigator.clipboard.writeText(whole).then(
              () => {
                copied(true)
                say.ok(t('cover.copied'))
              },
              () => copied(false),
            )
          }}
        >
          {t('cover.prompt.copyAll')}
        </Button>
        {cover.sent !== null && (
          <span className="text-2xs text-faint">
            {t('cover.prompt.sent', { time: formatStamp(cover.sent.at) })}
          </span>
        )}
        {view.sent_differs && (
          <span className="flex items-center gap-1 text-2xs text-warn">
            <AlertTriangle aria-hidden className="size-3" />
            {t('cover.prompt.changedSince')}
          </span>
        )}
      </footer>
    </Panel>
  )
}

/**
 * The person's own words for one block. On a built cover they close it; on
 * a cover written by hand they are the whole block, and edited here.
 */
function OwnWords({
  part,
  value,
  built,
  disabled,
  onCommit,
}: {
  part: Part
  value: string
  built: boolean
  disabled?: boolean
  onCommit: (text: string) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      {built && <span className="text-2xs text-faint">{t(`cover.prompt.own.${part}`)}</span>}
      <DraftText
        label={built ? t(`cover.prompt.own.${part}`) : t(`cover.prompt.part.${part}`)}
        value={value}
        rows={built ? 1 : 4}
        mono
        disabled={disabled}
        placeholder={built ? t('cover.prompt.ownPlaceholder') : undefined}
        onCommit={onCommit}
      />
    </div>
  )
}
