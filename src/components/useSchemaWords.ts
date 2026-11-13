import { useTranslation } from 'react-i18next'
import type { SchemaFieldsWords } from '@/components/ui/schema-fields'

/**
 * Every word dowel's SchemaFields can show, in the window's language: it has
 * none of its own, so that a form drawn from a schema says what the rest of
 * the window says.
 */
export function useSchemaWords(): SchemaFieldsWords {
  const { t } = useTranslation()
  return {
    issue: (issue) => (issue.code === 'required' ? t('fields.required') : t('fields.invalid')),
    none: t('fields.choiceNone'),
    choose: t('fields.choose'),
    pickDate: t('work.noDate'),
    previousMonth: t('calendar.previousMonth'),
    nextMonth: t('calendar.nextMonth'),
    clearDate: t('fields.clearDate'),
    showSecret: t('fields.showSecret'),
    hideSecret: t('fields.hideSecret'),
    removeTag: (tag) => t('work.removeTag', { tag }),
  }
}
