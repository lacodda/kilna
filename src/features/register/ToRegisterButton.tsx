import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ListPlus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { READING_TEXT } from '@/features/work/tabs/versions/ReadingText'
import { NewTermDialog } from '@/features/register/NewTermDialog'

/**
 * "To the register": the word selected in a version, entered as a term of the
 * register of repeats (ADR 0044).
 *
 * The selection is read when the button is pressed, from the text being read
 * or from the box it is being written in - a writer who notices a word leaned
 * on again should not have to put the pen down first. The button never takes
 * the focus from the text, so pressing it does not lose the selection. With
 * nothing selected the form opens empty.
 */
export function ToRegisterButton() {
  const { t } = useTranslation()
  const [word, setWord] = useState<string | null>(null)

  const selected = (): string => {
    const active = document.activeElement
    if (active instanceof HTMLTextAreaElement) {
      return active.value.slice(active.selectionStart, active.selectionEnd).trim()
    }
    const selection = window.getSelection()
    if (selection === null || selection.rangeCount === 0) return ''
    const anchor = selection.anchorNode
    const inside =
      anchor !== null &&
      (anchor instanceof Element ? anchor : anchor.parentElement)?.closest(`[${READING_TEXT}]`) !==
        null
    return inside ? selection.toString().trim() : ''
  }

  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        title={t('register.toRegisterHint')}
        onMouseDown={(event) => event.preventDefault()}
        onClick={() => setWord(selected())}
      >
        <ListPlus aria-hidden />
        <span className="@max-2xl:sr-only">{t('register.toRegister')}</span>
      </Button>
      <NewTermDialog
        open={word !== null}
        onOpenChange={(open) => {
          if (!open) setWord(null)
        }}
        word={word ?? ''}
      />
    </>
  )
}
