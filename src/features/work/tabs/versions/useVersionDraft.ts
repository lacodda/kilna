import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Version } from '@/lib/api/types'
import { createVersion } from '@/lib/api/versions'
import { clearDraft, readDraft, writeDraft } from '@/lib/drafts'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'

interface Options {
  workId: string
  /** The role the form writes into: the lane that is open. */
  role: string
  /** The version the draft became. */
  onSaved: (version: Version) => void
  /** What a version written from nothing in this role starts from: the
   *  role's skeleton in the profile (v0.91), or nothing. */
  skeleton?: string
}

/**
 * The form for a version written from nothing or from a copy, and the draft
 * typed into it.
 *
 * Not the everyday way of revising - that is editing the open text, which
 * mints the next revision by itself. The form is for the moments a revision
 * needs a decision first: the role has no version yet, a version wants a
 * name, or the person asked for a copy of one to work on.
 *
 * Drafts belong to the work and the role they were typed under, and outlive
 * the window (`lib/drafts`): putting the form away, opening another version
 * or switching roles loses nothing, and the form comes back with the text.
 */
export function useVersionDraft({ workId, role, onSaved, skeleton = '' }: Options) {
  const { t } = useTranslation()

  const [composing, setComposing] = useState(false)
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  // The version the draft was copied from, when it was; recorded on the saved
  // version as its parent, and shown beside the draft while it is written.
  const [derivedFrom, setDerivedFrom] = useState<string | null>(null)
  const typed = drafts[role] ?? readDraft(workId, role)
  // A version from nothing starts from the role's skeleton - a lyric's parts,
  // a chapter's plan - until something is typed. Not kept as a draft until it
  // is changed: an untouched skeleton is the form, not the person's words,
  // and clearing it leaves it cleared rather than bringing it back.
  const fromSkeleton = drafts[role] === undefined && typed === '' && derivedFrom === null
  const draft = fromSkeleton ? skeleton : typed
  const setDraftOf = (key: string, body: string) => {
    setDrafts((all) => ({ ...all, [key]: body }))
    writeDraft(workId, key, body)
  }
  const [label, setLabel] = useState('')
  const [makeCurrent, setMakeCurrent] = useState(true)

  const save = useAppMutation({
    mutationFn: () =>
      createVersion(workId, {
        role,
        body: draft,
        label: label.trim() === '' ? undefined : label.trim(),
        make_current: makeCurrent,
        parent_version_id: derivedFrom ?? undefined,
      }),
    failure: 'toast.versionSaveFailed',
    refresh: refresh.version(workId),
    onSuccess: (version) => {
      // The draft became a version; there is nothing left to keep.
      setDrafts((all) => ({ ...all, [role]: '' }))
      clearDraft(workId, role)
      setLabel('')
      setDerivedFrom(null)
      setComposing(false)
      onSaved(version)
    },
  })

  return {
    composing,
    draft,
    setDraft: (body: string) => setDraftOf(role, body),
    label,
    setLabel,
    makeCurrent,
    setMakeCurrent,
    derivedFrom,
    /** Whether what the form shows is the role's skeleton, untouched. */
    fromSkeleton: fromSkeleton && skeleton !== '',
    saving: save.isPending,
    save: () => {
      if (draft.trim() !== '') save.mutate()
    },

    /** A version from nothing: the form, with whatever was kept - or, with
     *  nothing kept, the role's skeleton again. */
    begin: () => {
      setDerivedFrom(null)
      if (readDraft(workId, role) === '') {
        setDrafts((all) => Object.fromEntries(Object.entries(all).filter(([key]) => key !== role)))
      }
      setComposing(true)
    },

    /** Put the form away. The draft stays where it was typed. */
    close: () => {
      setComposing(false)
      setDerivedFrom(null)
    },

    /**
     * A copy of `source` in the form, to be worked on as the next version -
     * for a rewrite that should not start from the open text as it is. A
     * draft already in the form is displaced rather than guarded by a
     * confirmation, with one click to take it back.
     */
    derive: (source: Version) => {
      const displaced = drafts[source.role] ?? readDraft(workId, source.role)
      setDraftOf(source.role, source.body)
      setDerivedFrom(source.id)
      setComposing(true)

      if (displaced.trim() === '') say.ok(t('toast.versionDerived'))
      else
        say.undoable(t('toast.versionDerived'), t('versions.restoreDraft'), () => {
          setDraftOf(source.role, displaced)
          setDerivedFrom(null)
        })
    },
  }
}
