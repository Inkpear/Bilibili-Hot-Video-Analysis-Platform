import { useApi } from '../hooks/useApi'
import { fetchWordCloud } from '../api'
import { useDashboard } from '../context/DashboardContext'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import type { EChartsOption } from 'echarts'
import type { WordCloudSource } from '../types'

const SOURCE_LABELS: Record<WordCloudSource, string> = {
  title: '标题',
  comment: '评论',
  danmaku: '弹幕',
}

export default function WordCloud() {
  const { selectedCategory, wordCloudSource, setWordCloudSource } = useDashboard()
  const { data, loading } = useApi(
    () => fetchWordCloud(wordCloudSource, selectedCategory ?? undefined),
    [wordCloudSource, selectedCategory]
  )

  const option: EChartsOption = data?.words?.length
    ? ({
        tooltip: { show: true },
        series: [
          {
            type: 'wordCloud',
            shape: 'circle',
            sizeRange: [14, 48],
            rotationRange: [-30, 30],
            rotationStep: 15,
            gridSize: 8,
            drawOutOfBound: false,
            textStyle: {
              fontFamily: 'Noto Sans SC, sans-serif',
              fontWeight: 600,
              color: () => {
                const colors = ['#00A1D6', '#FB7299', '#23C4F4', '#FF9CB8', '#79C0FF', '#39D353']
                return colors[Math.floor(Math.random() * colors.length)]
              },
            },
            data: data.words.map((w) => ({ name: w.name, value: w.value })),
          },
        ],
      } as EChartsOption)
    : {}

  const extra = (
    <div className="flex gap-1">
      {(['title', 'comment', 'danmaku'] as const).map((s) => (
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
  )

  return (
    <Card title="词云" extra={extra} className="h-full">
      <ChartWrapper option={option} loading={loading} />
    </Card>
  )
}
