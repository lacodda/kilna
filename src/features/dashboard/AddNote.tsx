import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import { createFocusNote } from '@/lib/api/focus'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

/** One field, opened by a link rather than sitting on the screen unused. */
export function AddNote() {
  const { t } = useTranslation()
  const [body, setBody] = useState('')
  const [open, setOpen] = useState(false)

  const add = useAppMutation({
    mutationFn: (line: string) => createFocusNote({ body: line }),
    refresh: [keys.focusNotes],
    onSuccess: () => {
      setBody('')
      setOpen(false)
    },
  })

  const submit = () => {
    const line = body.trim()
    if (line !== '') add.mutate(line)
  }

  if (!open) {
    return (
      // `text-sm` is the line's; the link takes its size from it.
      <p className="text-sm">
        <Button variant="link" onClick={() => setOpen(true)}>
          <Plus aria-hidden />
          {t('focus.add')}
        </Button>
      </p>
    )
  }

  return (
    <div className="flex min-w-0 items-center gap-2">
      <Input
        autoFocus
        value={body}
        placeholder={t('focus.addPlaceholder')}
        onChange={(event) => setBody(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') submit()
          // Escape gives up on the line rather than leaving an open field
          // nobody asked for.
          if (event.key === 'Escape') {
            setBody('')
            setOpen(false)
          }
        }}
      />
      <Button size="sm" variant="soft" disabled={add.isPending} onClick={submit}>
        {t('focus.save')}
      </Button>
    </div>
  )
}
