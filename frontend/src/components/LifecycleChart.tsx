import { useState, useMemo } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchLifecycleTop, fetchLifecycle } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import Loading from './common/Loading'
import type { EChartsOption } from 'echarts'

const COLORS = ['#00A1D6', '#FB7299', '#39D353', '#F0883E', '#A371F7']

export default function LifecycleChart() {
  const { selectedBvid } = useDashboard()
  const [metric, setMetric] = useState<'view_count' | 'like_count'>('view_count')

  const topData = useApi(() => fetchLifecycleTop(5), [])
  const singleData = useApi(
    () => selectedBvid ? fetchLifecycle(selectedBvid) : Promise.resolve(null),
    [selectedBvid]
  )

  const loading = selectedBvid ? singleData.loading : topData.loading
  const hasLoaded = selectedBvid ? singleData.data !== undefined : topData.data !== undefined

  const videos = useMemo(() => {
    if (selectedBvid) {
      return singleData.data ? [singleData.data] : []
    }
    return topData.data?.videos ?? []
  }, [selectedBvid, singleData.data, topData.data])

  const allDates = useMemo(
    () => [...new Set(videos.flatMap((v) => v.timeline.map((t) => t.snapshot_date)))].sort(),
    [videos]
  )

  const option: EChartsOption = useMemo(() => {
    if (!videos.length) return {}
    return {
      grid: { left: 60, right: 24, top: 32, bottom: 32, containLabel: false },
      legend: {
        show: videos.length > 1,
        top: 0,
        textStyle: { color: '#8B949E', fontSize: 11 },
      },
      tooltip: {
        trigger: 'axis',
        formatter: (params: unknown) => {
          const items = Array.isArray(params) ? params : [params]
          const lines = (items as Array<{ seriesName: string; value: number; color: string; axisValue: string }>)
            .map((item) => `<span style="color:${item.color}">●</span> ${item.seriesName}: ${formatNumber(item.value)}`)
          return `<div style="font-size:12px">${(items[0] as { axisValue: string }).axisValue}<br/>${lines.join('<br/>')}</div>`
        },
      },
      xAxis: {
        type: 'category',
        data: allDates,
        axisLabel: { color: '#8B949E', fontSize: 10, rotate: 30 },
      },
      yAxis: {
        type: 'value',
        name: metric === 'view_count' ? '播放量' : '点赞数',
        nameTextStyle: { color: '#8B949E' },
        axisLabel: {
          color: '#8B949E',
          formatter: (v: number) => (v >= 10000 ? v / 10000 + 'w' : String(v)),
        },
        splitLine: { lineStyle: { color: '#21262D', type: 'dashed' } },
      },
      series: videos.map((v, i) => ({
        name: v.title.slice(0, 15),
        type: 'line' as const,
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        lineStyle: { color: COLORS[i % COLORS.length], width: 2 },
        itemStyle: { color: COLORS[i % COLORS.length] },
        areaStyle: {
          color: {
            type: 'linear' as const,
            x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: COLORS[i % COLORS.length] + '30' },
              { offset: 1, color: COLORS[i % COLORS.length] + '05' },
            ],
          },
        },
        data: allDates.map((d) => {
          const point = v.timeline.find((t) => t.snapshot_date === d)
          return point ? point[metric] : 0
        }),
      })),
    } as EChartsOption
  }, [videos, allDates, metric])

  const selectedTitle = selectedBvid && singleData.data ? singleData.data.title.slice(0, 12) : null

  const extra = (
    <div className="flex items-center gap-2">
      {selectedTitle && (
        <span className="px-1.5 py-0.5 text-[10px] rounded bg-bili-pink/20 text-bili-pink truncate max-w-[100px]">
          {selectedTitle}
        </span>
      )}
      <div className="flex gap-1">
        {[
          { value: 'view_count' as const, label: '播放量' },
          { value: 'like_count' as const, label: '点赞数' },
        ].map((m) => (
          <button
            key={m.value}
            onClick={() => setMetric(m.value)}
            className={`px-2 py-0.5 text-xs rounded transition-colors ${
              metric === m.value
                ? 'bg-bili-blue text-white'
                : 'text-text-muted hover:text-text-secondary'
            }`}
          >
            {m.label}
          </button>
        ))}
      </div>
    </div>
  )

  const showEmpty = !loading && hasLoaded && !videos.length

  return (
    <Card title="生命周期" extra={extra} className="h-full">
      {showEmpty ? (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          选择视频查看生命周期趋势
        </div>
      ) : loading ? (
        <Loading rows={5} />
      ) : (
        <ChartWrapper option={option} />
      )}
    </Card>
  )
}
