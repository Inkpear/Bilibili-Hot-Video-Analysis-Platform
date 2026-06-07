import { useMemo } from 'react'
import type { EChartsOption } from 'echarts'
import { fetchPartitionCompare } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { useApi } from '../hooks/useApi'
import { formatNumber, formatPercent } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'

type MetricKey =
  | 'hot_video_count'
  | 'avg_views'
  | 'total_views'
  | 'up_count'
  | 'avg_like_rate'
  | 'avg_coin_rate'
  | 'avg_fav_rate'

interface MetricConfig {
  key: MetricKey
  label: string
  format: (value: number) => string
}

const METRICS: MetricConfig[] = [
  { key: 'hot_video_count', label: '样本', format: (value) => String(value) },
  { key: 'avg_views', label: '均播', format: formatNumber },
  { key: 'total_views', label: '总播', format: formatNumber },
  { key: 'up_count', label: 'UP', format: (value) => String(value) },
  { key: 'avg_like_rate', label: '点赞率', format: formatPercent },
  { key: 'avg_coin_rate', label: '投币率', format: formatPercent },
  { key: 'avg_fav_rate', label: '收藏率', format: formatPercent },
]

function normalize(value: number, min: number, max: number) {
  if (max === min) return 0.5
  return (value - min) / (max - min)
}

function getCellColor(norm: number, selected: boolean) {
  const alpha = selected ? 0.28 + norm * 0.52 : 0.14 + norm * 0.38
  return selected ? `rgba(251, 114, 153, ${alpha.toFixed(3)})` : `rgba(0, 161, 214, ${alpha.toFixed(3)})`
}

export default function PartitionMetricMatrix() {
  const { data, loading } = useApi(fetchPartitionCompare)
  const { selectedCategory, setSelectedCategory } = useDashboard()

  const partitions = useMemo(
    () =>
      [...(data?.partitions ?? [])].sort((a, b) => {
        if (b.total_views !== a.total_views) return b.total_views - a.total_views
        return b.avg_views - a.avg_views
      }),
    [data]
  )

  const option = useMemo<EChartsOption>(() => {
    if (partitions.length === 0) return {}

    const ranges = Object.fromEntries(
      METRICS.map((metric) => {
        const values = partitions.map((partition) => partition[metric.key])
        return [metric.key, { min: Math.min(...values), max: Math.max(...values) }]
      })
    ) as Record<MetricKey, { min: number; max: number }>

    const seriesData = partitions.flatMap((partition, rowIndex) =>
      METRICS.map((metric, colIndex) => {
        const rawValue = partition[metric.key]
        const range = ranges[metric.key]
        const norm = normalize(rawValue, range.min, range.max)
        const isSelected = partition.category === selectedCategory

        return {
          value: [colIndex, rowIndex, norm, rawValue],
          category: partition.category,
          metricKey: metric.key,
          itemStyle: {
            color: getCellColor(norm, isSelected),
            borderColor: isSelected ? 'rgba(251, 114, 153, 0.95)' : 'rgba(13, 17, 23, 0.65)',
            borderWidth: isSelected ? 2 : 1,
            borderRadius: 7,
          },
          label: {
            color: isSelected || norm > 0.5 ? '#E6EDF3' : '#8B949E',
          },
        }
      })
    )

    return {
      animationDuration: 380,
      visualMap: {
        show: false,
        min: 0,
        max: 1,
        dimension: 2,
      },
      grid: {
        left: 72,
        right: 8,
        top: 6,
        bottom: 8,
        containLabel: true,
      },
      tooltip: {
        trigger: 'item',
        backgroundColor: '#161B22',
        borderColor: 'rgba(0, 161, 214, 0.18)',
        formatter: (params: unknown) => {
          const point = params as {
            data: { category: string; metricKey: MetricKey; value: [number, number, number, number] }
          }
          const metric = METRICS.find((item) => item.key === point.data.metricKey)
          if (!metric) return ''

          return [
            `<div style="font-size:12px;">`,
            `<div style="color:#E6EDF3;font-weight:600;margin-bottom:4px;">${point.data.category}</div>`,
            `<div>${metric.label}: <span style="color:#79C0FF;font-family:JetBrains Mono, monospace;">${metric.format(point.data.value[3])}</span></div>`,
            `<div style="color:#8B949E;margin-top:4px;">色深表示同列指标下的相对高低</div>`,
            `</div>`,
          ].join('')
        },
      },
      xAxis: {
        type: 'category',
        position: 'top',
        data: METRICS.map((metric) => metric.label),
        axisTick: { show: false },
        axisLine: { show: false },
        axisLabel: {
          color: '#8B949E',
          fontSize: 9,
          interval: 0,
          margin: 8,
        },
      },
      yAxis: {
        type: 'category',
        data: partitions.map((partition) => partition.category),
        axisTick: { show: false },
        axisLine: { show: false },
        axisLabel: {
          color: '#E6EDF3',
          fontSize: 10,
          margin: 8,
        },
      },
      series: [
        {
          type: 'heatmap',
          progressive: 0,
          data: seriesData,
          label: {
            show: true,
            fontSize: 8,
            lineHeight: 10,
            formatter: (params: unknown) => {
              const point = params as { data: { metricKey: MetricKey; value: [number, number, number, number] } }
              const metric = METRICS.find((item) => item.key === point.data.metricKey)
              return metric ? metric.format(point.data.value[3]) : ''
            },
          },
          emphasis: {
            itemStyle: {
              shadowBlur: 14,
              shadowColor: 'rgba(0, 0, 0, 0.22)',
            },
          },
        },
      ],
    }
  }, [partitions, selectedCategory])

  const currentLabel = selectedCategory ?? '自动聚焦头部分区'

  return (
    <Card
      title="分区指标矩阵"
      extra={<span className="text-[10px] text-text-muted">点击单元格切换分区</span>}
      className="h-full"
    >
      <div className="flex h-full min-h-0 flex-col gap-2">
        <div className="flex items-center justify-between gap-2 text-[10px]">
          <span className="rounded-full bg-bili-blue/15 px-2 py-0.5 text-bili-blue">{currentLabel}</span>
          <span className="text-text-muted">色深 = 同列相对位置</span>
        </div>

        {partitions.length > 0 ? (
          <div className="flex-1 min-h-0">
            <ChartWrapper
              option={option}
              loading={loading}
              onEvents={{
                click: (params: unknown) => {
                  const point = params as { data?: { category?: string } }
                  const category = point.data?.category
                  if (!category) return
                  setSelectedCategory(category === selectedCategory ? null : category)
                },
              }}
            />
          </div>
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-text-muted">暂无分区对比数据</div>
        )}
      </div>
    </Card>
  )
}
