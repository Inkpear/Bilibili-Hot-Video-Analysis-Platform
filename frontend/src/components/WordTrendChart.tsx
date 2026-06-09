import { useMemo, useState } from 'react'
import type { EChartsOption } from 'echarts'
import { fetchWordTrend } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { useApi } from '../hooks/useApi'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import Loading from './common/Loading'
import type { WordTrendItem, WordCloudSource } from '../types'

const COLORS = [
  '#00A1D6',
  '#FB7299',
  '#39D353',
  '#F0883E',
  '#A371F7',
  '#79C0FF',
  '#23C4F4',
  '#E6EDF3',
  '#8B949E',
  '#C9D1D9',
]

const SOURCE_LABELS: Record<WordCloudSource, string> = {
  title: '标题',
  comment: '评论',
  danmaku: '弹幕',
}

function groupByWord(items: WordTrendItem[]): Map<string, WordTrendItem[]> {
  const map = new Map<string, WordTrendItem[]>()
  for (const item of items) {
    const list = map.get(item.word)
    if (list) {
      list.push(item)
    } else {
      map.set(item.word, [item])
    }
  }
  return map
}

function calcWordTotal(items: WordTrendItem[]): number {
  return items.reduce((sum, i) => sum + i.total_count, 0)
}

function formatDate(yyyymmdd: string): string {
  return `${yyyymmdd.slice(4, 6)}-${yyyymmdd.slice(6, 8)}`
}

const SOURCE_LIST: WordCloudSource[] = ['title', 'comment', 'danmaku']

export default function WordTrendChart() {
  const {
    selectedWord,
    selectedCategory,
    selectedBvid,
    selectedSnapshotDate,
    wordCloudSource,
    setWordCloudSource,
    setSelectedWord,
  } = useDashboard()

  const [manualWord, setManualWord] = useState('')

  const handleManualSearch = () => {
    const trimmed = manualWord.trim()
    if (trimmed) setSelectedWord(trimmed)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleManualSearch()
  }

  const { data, loading } = useApi(
    () =>
      fetchWordTrend(
        selectedWord ?? undefined,
        selectedCategory ?? undefined,
        wordCloudSource,
        selectedBvid ?? undefined,
        selectedSnapshotDate,
      ),
    [selectedWord, selectedCategory, wordCloudSource, selectedBvid, selectedSnapshotDate],
  )

  const wordGroups = useMemo(() => {
    if (!data?.items?.length) return null
    return groupByWord(data.items)
  }, [data])

  const topWords = useMemo(() => {
    if (!wordGroups) return []
    if (selectedWord) {
      return wordGroups.has(selectedWord) ? [selectedWord] : []
    }
    return [...wordGroups.entries()]
      .map(([word, items]) => ({ word, total: calcWordTotal(items) }))
      .sort((a, b) => b.total - a.total)
      .slice(0, 10)
      .map((w) => w.word)
  }, [wordGroups, selectedWord])

  const allDates = useMemo(() => {
    if (!data?.items) return []
    return [...new Set(data.items.map((i) => i.snapshot_date))].sort()
  }, [data])

  const hasMultipleWords = topWords.length > 1

  const option: EChartsOption = useMemo(() => {
    if (!topWords.length || !allDates.length) return {}

    return {
      grid: {
        left: 56,
        right: hasMultipleWords ? '22%' : 20,
        top: 16,
        bottom: 36,
        containLabel: false,
      },
      legend: {
        show: hasMultipleWords,
        orient: 'vertical',
        right: '2%',
        top: 'middle',
        itemWidth: 12,
        itemHeight: 8,
        itemGap: 14,
        textStyle: {
          color: '#8B949E',
          fontSize: 10,
          width: 90,
          overflow: 'truncate',
          ellipsis: '…',
        },
      },
      tooltip: {
        trigger: 'axis',
        formatter: (params: unknown) => {
          const items = Array.isArray(params) ? params : [params]
          const cast = items as Array<{
            seriesName: string
            value: number
            color: string
            axisValue: string
          }>
          const lines = cast.map(
            (item) =>
              `<span style="color:${item.color}">●</span> ${item.seriesName}: ${formatNumber(item.value)}`,
          )
          return `<div style="font-size:12px">${cast[0].axisValue}<br/>${lines.join('<br/>')}</div>`
        },
      },
      xAxis: {
        type: 'category',
        data: allDates.map(formatDate),
        axisLabel: { color: '#8B949E', fontSize: 10 },
        boundaryGap: false,
      },
      yAxis: {
        type: 'value',
        axisLabel: {
          color: '#8B949E',
          fontSize: 10,
          formatter: (v: number) => formatNumber(v),
        },
        splitLine: { lineStyle: { color: '#21262D', type: 'dashed' } },
      },
      series: topWords.map((word, i) => {
        const color = COLORS[i % COLORS.length]
        const items = wordGroups?.get(word) ?? []
        const dateMap = new Map(items.map((it) => [it.snapshot_date, it.total_count]))

        return {
          name: word,
          type: 'line' as const,
          smooth: true,
          symbol: 'circle',
          symbolSize: selectedWord ? 6 : 5,
          lineStyle: {
            color,
            width: selectedWord ? 3 : 2,
          },
          itemStyle: { color },
          areaStyle: {
            color: {
              type: 'linear' as const,
              x: 0,
              y: 0,
              x2: 0,
              y2: 1,
              colorStops: [
                { offset: 0, color: color + '30' },
                { offset: 1, color: color + '05' },
              ],
            },
          },
          data: allDates.map((d) => dateMap.get(d) ?? 0),
        }
      }),
    } as EChartsOption
  }, [topWords, allDates, hasMultipleWords, wordGroups, selectedWord])

  const hasData = (data?.items?.length ?? 0) > 0

  const extra = (
    <div className="flex items-center gap-2">
      <div className="flex items-center gap-1">
        <input
          type="text"
          value={manualWord}
          onChange={(e) => setManualWord(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="搜索词..."
          className="w-20 rounded-md border border-border-default bg-bg-primary px-2 py-0.5 text-xs text-text-primary placeholder:text-text-muted focus:border-bili-blue focus:outline-none"
        />
        <button
          onClick={handleManualSearch}
          className="rounded-md bg-bili-blue px-2 py-0.5 text-xs text-white hover:bg-bili-blue/80 transition-colors"
        >
          追踪
        </button>
      </div>
      {selectedWord && (
        <button
          onClick={() => { setSelectedWord(null); setManualWord('') }}
          className="px-1.5 py-0.5 text-[10px] rounded bg-bili-pink/20 text-bili-pink hover:bg-bili-pink/30 transition-colors"
        >
          {selectedWord} ✕
        </button>
      )}
      <div className="flex gap-1">
        {SOURCE_LIST.map((s) => (
          <button
            key={s}
            onClick={() => setWordCloudSource(s)}
            className={`px-2 py-0.5 text-xs rounded transition-colors ${
              wordCloudSource === s
                ? 'bg-bili-blue text-white'
                : 'text-text-muted hover:text-text-secondary'
            }`}
          >
            {SOURCE_LABELS[s]}
          </button>
        ))}
      </div>
    </div>
  )

  return (
    <Card title="词频趋势" extra={extra} className="h-full">
      {loading ? (
        <Loading rows={5} />
      ) : hasData ? (
        <ChartWrapper option={option} />
      ) : (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          暂无词频趋势数据
        </div>
      )}
    </Card>
  )
}
