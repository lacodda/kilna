import { useTranslation } from 'react-i18next'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { AlertTriangle, FolderOpen } from 'lucide-react'
import type { CoverProblem, CoverReference } from '@/lib/api/types'
import { fileSrc } from '@/lib/api/assets'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Panel, SectionLabel } from '@/components/ui/panel'

/**
 * What the constructor has to say about a cover: a part that is chosen and
 * cannot be used as it is - a hero with no description, a slot with nothing
 * in it, a mark with no file to lay over.
 */
export function CoverProblems({ problems }: { problems: CoverProblem[] }) {
  const { t } = useTranslation()
  if (problems.length === 0) return null
  return (
    <ul aria-label={t('cover.problems')} className="flex flex-col gap-1">
      {problems.map((problem, index) => (
        <li key={index} className="flex items-start gap-1.5 text-xs text-warn">
          <AlertTriangle aria-hidden className="mt-0.5 size-3.5 shrink-0" />
          {t(`cover.problem.${problem.kind}`, problem)}
        </li>
      ))}
    </ul>
  )
}

/**
 * The files that go to the generator with the prompt: the hero's likeness,
 * the mark to draw in the picture's style, the style's references. A
 * generator's page takes them as uploads, so each is one click from its
 * folder.
 */
export function CoverReferences({ references }: { references: CoverReference[] }) {
  const { t } = useTranslation()
  if (references.length === 0) return null
  return (
    <Panel className="flex flex-col gap-2 p-3">
      <header className="flex flex-wrap items-baseline gap-2">
        <SectionLabel>{t('cover.references.title')}</SectionLabel>
        <span className="text-2xs text-faint">{t('cover.references.hint')}</span>
      </header>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(5.5rem,1fr))] gap-2">
        {references.map((reference) => (
          <li key={`${reference.role}:${reference.asset.id}`} className="flex flex-col gap-1">
            <img
              src={fileSrc(reference.asset.path)}
              alt={reference.asset.original_name ?? ''}
              className="aspect-square w-full rounded-md bg-soft object-cover"
              draggable={false}
            />
            <span className="flex items-center gap-1">
              <span className="min-w-0 flex-1 truncate text-2xs text-faint">
                {t(`cover.references.role.${reference.role}`)}
              </span>
              <Button
                variant="icon"
                size="icon-xs"
                aria-label={t('cover.references.show')}
                title={t('cover.references.show')}
                onClick={() => {
                  revealItemInDir(reference.asset.path).catch((error: unknown) => say.failed(error))
                }}
              >
                <FolderOpen />
              </Button>
            </span>
          </li>
        ))}
      </ul>
    </Panel>
  )
}
