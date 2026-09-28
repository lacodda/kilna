import { useTranslation } from 'react-i18next'
import { Check, ChevronDown } from 'lucide-react'
import { ALL_COLUMNS, REQUIRED_COLUMN, toggleColumn, type ColumnId } from '@/lib/catalogue'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Menu,
  MenuCheckboxIndicator,
  MenuCheckboxItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from '@/components/ui/menu'
import { ReorderGrip, ReorderIndicator, useReorder } from '@/components/ui/reorderable-list'
import { COLUMN_SPECS } from './columns'

interface Props {
  columns: ColumnId[]
  onChange: (next: ColumnId[]) => void
  onMove: (id: ColumnId, to: number) => void
}

/**
 * Which columns the table draws, and in what order.
 *
 * A menu rather than a dialog: choosing columns is something a person does
 * while looking at the table, and a dialog would cover the thing being changed.
 *
 * The shown columns come first, in the order the table draws them, each with
 * a grip; the hidden ones follow below a line. So the top of the menu is a
 * picture of the header, and dragging a row up or down in it is dragging the
 * column left or right - a menu is the one place the columns are already
 * standing in a list.
 */
export function ColumnPicker({ columns, onChange, onMove }: Props) {
  const { t } = useTranslation()
  const reorder = useReorder<ColumnId>({ order: columns, onMove })
  const hidden = ALL_COLUMNS.filter((id) => !columns.includes(id))

  return (
    <Menu>
      <MenuTrigger render={<Button size="sm" variant="ghost" title={t('catalogue.columns')} />}>
        {t('catalogue.columnsButton')}
        <ChevronDown aria-hidden />
      </MenuTrigger>

      <MenuPopup align="end">
        <div {...reorder.listProps} className="relative">
          {columns.map((id) => (
            <MenuCheckboxItem
              key={id}
              checked
              // The title carries the row's identity and its link. Offered as
              // permanently ticked rather than left out of the list, so its
              // absence is a statement instead of an oversight. It can still
              // be dragged: pointer events are given back to it for the grip,
              // which is the one thing a disabled item here may do.
              disabled={id === REQUIRED_COLUMN}
              // The menu stays open: turning columns on and off is a comparison,
              // and closing after each would make a five-column change five trips.
              closeOnClick={false}
              onCheckedChange={() => onChange(toggleColumn(columns, id))}
              title={t('catalogue.moveHint')}
              className={cn(
                'data-[disabled]:pointer-events-auto',
                reorder.dragging === id && 'opacity-50',
              )}
              {...reorder.rowProps(id)}
            >
              <MenuCheckboxIndicator>
                <Check className="size-3.5" aria-hidden />
              </MenuCheckboxIndicator>
              <span className="flex-1">{t(COLUMN_SPECS[id].label)}</span>
              <ReorderGrip
                {...reorder.gripProps(id)}
                title={t('catalogue.moveColumn', { column: t(COLUMN_SPECS[id].label) })}
              />
            </MenuCheckboxItem>
          ))}
          <ReorderIndicator offset={reorder.slotOffset} />
        </div>

        {hidden.length > 0 && (
          <>
            <MenuSeparator />
            {hidden.map((id) => (
              <MenuCheckboxItem
                key={id}
                checked={false}
                closeOnClick={false}
                onCheckedChange={() => onChange(toggleColumn(columns, id))}
              >
                <MenuCheckboxIndicator>
                  <Check className="size-3.5" aria-hidden />
                </MenuCheckboxIndicator>
                {t(COLUMN_SPECS[id].label)}
              </MenuCheckboxItem>
            ))}
          </>
        )}
      </MenuPopup>
    </Menu>
  )
}
