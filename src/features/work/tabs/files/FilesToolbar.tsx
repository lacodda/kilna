import { useTranslation } from 'react-i18next'
import {
  ArrowDownWideNarrow,
  ArrowUpNarrowWide,
  LayoutGrid,
  LayoutList,
  Rows3,
  Square,
} from 'lucide-react'
import {
  CATEGORIES,
  SORTS,
  VIEWS,
  type FileCategory,
  type FilesLook,
  type FileView,
  type GroupBy,
  type SortBy,
} from '@/lib/fileList'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Select } from '@/components/AppSelect'

interface Props {
  look: FilesLook
  onChange: (look: FilesLook) => void
  /** How many files there are of each category, attached and on disk
   *  together: a chip says what ticking it would leave. */
  counts: Record<FileCategory, number>
}

const VIEW_ICON: Record<FileView, typeof Square> = {
  large: Square,
  tiles: LayoutGrid,
  list: LayoutList,
  table: Rows3,
}

/**
 * How the Files tab is arranged (FR-14): which categories show, what the
 * files are gathered by, how they are sorted, and which of the four views
 * draws them. One bar for the attached files and the folder on disk alike -
 * they are looked through together, and two bars would be two answers to
 * one question.
 */
export function FilesToolbar({ look, onChange, counts }: Props) {
  const { t } = useTranslation()
  const Direction = look.descending ? ArrowDownWideNarrow : ArrowUpNarrowWide
  const direction = look.descending ? t('files.sort.descending') : t('files.sort.ascending')

  return (
    <div
      role="toolbar"
      aria-label={t('files.arrange')}
      className="flex w-full flex-wrap items-center gap-x-3 gap-y-2"
    >
      <ChipGroup
        multiple
        aria-label={t('files.onlyLabel')}
        value={look.only}
        onValueChange={(only) =>
          onChange({ ...look, only: CATEGORIES.filter((one) => only.includes(one)) })
        }
      >
        {CATEGORIES.map((category) => (
          <Chip key={category} value={category}>
            {t(`files.category.${category}`)}
            <span className="font-mono text-2xs">{counts[category]}</span>
          </Chip>
        ))}
      </ChipGroup>

      <span className="ml-auto flex flex-wrap items-center gap-2">
        <SegmentedControl
          aria-label={t('files.groupLabel')}
          value={look.group}
          onValueChange={(group) => onChange({ ...look, group: group as GroupBy })}
        >
          <Segment value="type">{t('files.groupBy.type')}</Segment>
          <Segment value="place">{t('files.groupBy.place')}</Segment>
        </SegmentedControl>

        <span className="flex items-center gap-1">
          <Select
            className="w-36"
            aria-label={t('files.sortLabel')}
            value={look.sort}
            onChange={(sort) => onChange({ ...look, sort: sort as SortBy })}
            options={SORTS.map((sort) => ({ value: sort, label: t(`files.sort.${sort}`) }))}
          />
          <Button
            variant="icon"
            size="icon-sm"
            aria-label={direction}
            title={direction}
            aria-pressed={look.descending}
            onClick={() => onChange({ ...look, descending: !look.descending })}
          >
            <Direction aria-hidden />
          </Button>
        </span>

        <SegmentedControl
          aria-label={t('files.viewLabel')}
          value={look.view}
          onValueChange={(view) => onChange({ ...look, view: view as FileView })}
        >
          {VIEWS.map((view) => {
            const Icon = VIEW_ICON[view]
            return (
              <Segment key={view} value={view} aria-label={t(`files.view.${view}`)}>
                <span title={t(`files.view.${view}`)} className="flex">
                  <Icon aria-hidden className="size-3.5" />
                </span>
              </Segment>
            )
          })}
        </SegmentedControl>
      </span>
    </div>
  )
}
