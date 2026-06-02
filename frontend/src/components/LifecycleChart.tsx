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

  const hasLegend = videos.length > 1

  const option: EChartsOption = useMemo(() => {
    if (!videos.length) return {}
    return {
      grid: {
        left: 56,
        right: hasLegend ? '22%' : 20,
        top: 16,
        bottom: 36,
        containLabel: false,
      },
      legend: {
        show: hasLegend,
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
          const lines = (items as Array<{ seriesName: string; value: number; color: string; axisValue: string }>)
            .map((item) => `<span style="color:${item.color}">●</span> ${item.seriesName}: ${formatNumber(item.value)}`)
          return `<div style="font-size:12px">${(items[0] as { axisValue: string }).axisValue}<br/>${lines.join('<br/>')}</div>`
        },
      },
      xAxis: {
        type: 'category',
        data: allDates,
        axisLabel: { color: '#8B949E', fontSize: 10, rotate: 30 },
        boundaryGap: false,
      },
      yAxis: {
        type: 'value',
        name: metric === 'view_count' ? '播放量' : '点赞数',
        nameTextStyle: { color: '#8B949E', fontSize: 10, padding: [0, 0, 0, -20] },
        axisLabel: {
          color: '#8B949E',
          fontSize: 10,
          formatter: (v: number) => {
            if (v >= 100000000) return (v / 100000000).toFixed(0) + '亿'
            if (v >= 10000) return (v / 10000).toFixed(0) + 'w'
            return String(v)
          },
        },
        splitLine: { lineStyle: { color: '#21262D', type: 'dashed' } },
      },
      series: videos.map((v, i) => ({
        name: v.title.slice(0, 10),
        type: 'line' as const,
        smooth: true,
        symbol: 'circle',
        symbolSize: 5,
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
  }, [videos, allDates, metric, hasLegend])

  const selectedTitle = selectedBvid && singleData.data ? singleData.data.title.slice(0, 12) : null
  const noLifecycleData = selectedBvid && !loading && !singleData.data

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

  return (
    <Card title="生命周期" extra={extra} className="h-full">
      {loading ? (
        <Loading rows={5} />
      ) : videos.length > 0 ? (
        <div key={selectedBvid ?? 'top'} className="h-full animate-content-switch">
          <ChartWrapper option={option} />
        </div>
      ) : noLifecycleData ? (
        <div className="flex flex-col items-center justify-center h-full gap-2 text-text-muted text-sm animate-fade-in">
          <span>该视频暂无生命周期数据</span>
          <span className="text-xs">（需要多轮采集后才有趋势）</span>
        </div>
      ) : (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          选择视频查看生命周期趋势
        </div>
      )}
    </Card>
  )
}
