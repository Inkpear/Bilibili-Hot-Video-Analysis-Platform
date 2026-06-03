import { useCallback, useEffect, useRef } from 'react'
import 'echarts-wordcloud'
import { useApi } from '../hooks/useApi'
import { fetchWordCloud } from '../api'
import { useDashboard } from '../context/DashboardContext'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import Loading from './common/Loading'
import type { EChartsOption } from 'echarts'
import type { WordCloudSource } from '../types'

const SOURCE_LABELS: Record<WordCloudSource, string> = {
  title: '标题',
  comment: '评论',
  danmaku: '弹幕',
}

export default function WordCloud() {
  const {
    selectedCategory, selectedBvid, selectedWord, setSelectedWord,
    wordCloudSource, setWordCloudSource, carousel, markWordCloudShown,
  } = useDashboard()

  const switchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!carousel.on || !selectedBvid) {
      if (switchTimerRef.current) clearTimeout(switchTimerRef.current)
      switchTimerRef.current = null
      return
    }

    const { commentShown, danmakuShown } = carousel.wordCloud
    if (commentShown && danmakuShown) return

    if (!commentShown) {
      setWordCloudSource('comment')
      switchTimerRef.current = setTimeout(() => {
        markWordCloudShown('comment')
        setWordCloudSource('danmaku')
        switchTimerRef.current = setTimeout(() => {
          markWordCloudShown('danmaku')
        }, 5000)
      }, 5000)
    } else if (!danmakuShown) {
      setWordCloudSource('danmaku')
      switchTimerRef.current = setTimeout(() => {
        markWordCloudShown('danmaku')
      }, 5000)
    }

    return () => {
      if (switchTimerRef.current) clearTimeout(switchTimerRef.current)
    }
  }, [carousel.on, selectedBvid, carousel.wordCloud.commentShown, carousel.wordCloud.danmakuShown]) // eslint-disable-line react-hooks/exhaustive-deps

  const hasVideo = !!selectedBvid
  const effectiveSource = hasVideo
    ? (wordCloudSource === 'title' ? 'comment' : wordCloudSource)
    : wordCloudSource

  const categoryParam = selectedCategory ?? undefined
  const bvidParam = selectedBvid ?? undefined

  const { data, loading } = useApi(
    () => fetchWordCloud(effectiveSource, categoryParam, bvidParam),
    [effectiveSource, categoryParam, bvidParam]
  )

  const hasWords = (data?.words?.length ?? 0) > 0

  const handleChartClick = useCallback(
    (params: unknown) => {
      const p = params as { name?: string }
      if (p.name) setSelectedWord(p.name)
    },
    [setSelectedWord]
  )

  const option: EChartsOption = hasWords
    ? ({
        tooltip: {
          show: true,
          formatter: (params: unknown) => {
            const p = params as { name: string; value: number }
            return `<b>${p.name}</b><br/>出现 ${p.value} 次`
          },
        },
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
              color: (params: { name: string }) => {
                if (params.name === selectedWord) return '#FB7299'
                const colors = ['#00A1D6', '#23C4F4', '#79C0FF', '#39D353', '#F0883E', '#A371F7']
                const hash = params.name.split('').reduce((a, c) => a + c.charCodeAt(0), 0)
                return colors[hash % colors.length]
              },
            },
            data: data!.words.map((w) => ({
              name: w.name,
              value: w.value,
              textStyle: w.name === selectedWord
                ? { textShadowBlur: 10, textShadowColor: 'rgba(251,114,153,0.5)' }
                : undefined,
            })),
          },
        ],
      } as EChartsOption)
    : {}

  const wcDone = carousel.on && carousel.wordCloud.commentShown && carousel.wordCloud.danmakuShown
  const sources: WordCloudSource[] = hasVideo ? ['comment', 'danmaku'] : ['title', 'comment', 'danmaku']

  const extra = (
    <div className="flex items-center gap-2">
      {selectedWord && (
        <button
          onClick={() => setSelectedWord(null)}
          className="px-1.5 py-0.5 text-[10px] rounded bg-bili-pink/20 text-bili-pink hover:bg-bili-pink/30 transition-colors"
        >
          {selectedWord} ✕
        </button>
      )}
      <div className="flex gap-1">
        {sources.map((s) => (
          <button
            key={s}
            onClick={() => { if (!carousel.on) setWordCloudSource(s) }}
            className={`px-2 py-0.5 text-xs rounded transition-colors ${
              effectiveSource === s
                ? 'bg-bili-blue text-white'
                : 'text-text-muted hover:text-text-secondary'
            } ${carousel.on ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            {SOURCE_LABELS[s]}
            {carousel.on && s !== 'title' && carousel.wordCloud[`${s}Shown`] && (
              <span className="ml-1 text-positive">✓</span>
            )}
          </button>
        ))}
        {wcDone && (
          <span className="px-1.5 py-0.5 text-[10px] rounded bg-positive/20 text-positive">完成</span>
        )}
      </div>
    </div>
  )

  const title = hasVideo ? '视频词云' : '词云'

  return (
    <Card title={title} extra={extra} className="h-full">
      {loading ? (
        <Loading rows={4} />
      ) : hasWords ? (
        <ChartWrapper option={option} onEvents={{ click: handleChartClick }} />
      ) : (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          暂无词云数据
        </div>
      )}
    </Card>
  )
}
