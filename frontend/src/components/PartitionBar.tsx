import { useState } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchPartitionCompare } from '../api'
import { useDashboard } from '../context/DashboardContext'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import type { EChartsOption } from 'echarts'

type MetricKey = 'total_likes' | 'total_coins' | 'total_favorites' | 'total_views'

const METRICS: { key: MetricKey; label: string; format: (v: number) => string }[] = [
  { key: 'total_likes', label: '总点赞', format: (v) => v >= 10000 ? (v / 10000).toFixed(1) + '万' : String(v) },
  { key: 'total_coins', label: '总投币', format: (v) => v >= 10000 ? (v / 10000).toFixed(1) + '万' : String(v) },
  { key: 'total_favorites', label: '总收藏', format: (v) => v >= 10000 ? (v / 10000).toFixed(1) + '万' : String(v) },
  { key: 'total_views', label: '总播放', format: (v) => v >= 100000000 ? (v / 100000000).toFixed(1) + '亿' : (v / 10000).toFixed(0) + '万' },
]

export default function PartitionBar() {
  const { selectedSnapshotDate } = useDashboard()
  const { selectedCategory, setSelectedCategory } = useDashboard()
  const { data, loading } = useApi(() => fetchPartitionCompare(selectedSnapshotDate), [selectedSnapshotDate])
  const [metric, setMetric] = useState<MetricKey>('total_likes')

  const partitions = [...(data?.partitions ?? [])].sort((a, b) => b[metric] - a[metric])

  const option: EChartsOption = partitions.length
    ? {
        grid: { left: 65, right: 16, top: 8, bottom: 24 },
        xAxis: {
          type: 'value',
          axisLabel: {
            color: '#8B949E',
            fontSize: 10,
            formatter: (v: number) => v >= 100000000 ? (v / 100000000).toFixed(1) + '亿' : v >= 10000 ? (v / 10000).toFixed(1) + 'w' : String(v),
          },
        },
        yAxis: {
          type: 'category',
          inverse: true,
          data: partitions.map((p) => p.category),
          axisLabel: { color: '#E6EDF3', fontSize: 11 },
        },
        tooltip: {
          trigger: 'axis',
          axisPointer: { type: 'shadow' },
          formatter: (params: any) => {
            const p = partitions[params[0]?.dataIndex]
            if (!p) return ''
            return `<div style="font-size:12px">
              <div style="color:#E6EDF3;font-weight:600;margin-bottom:4px">${p.category}</div>
              <div>${METRICS.find(m => m.key === metric)?.label}: ${METRICS.find(m => m.key === metric)?.format(p[metric])}</div>
              <div style="color:#8B949E">视频: ${p.hot_video_count} | UP: ${p.up_count}</div>
              <div style="color:#8B949E">最大播放: ${p.top_view_count >= 10000 ? (p.top_view_count/10000).toFixed(0)+'万' : p.top_view_count}</div>
            </div>`
          },
        },
        series: [
          {
            type: 'bar',
            barWidth: '60%',
            data: partitions.map((p) => ({
              value: p[metric],
              itemStyle: {
                color: p.category === selectedCategory ? '#FB7299' : '#00A1D6',
                borderRadius: [0, 4, 4, 0],
              },
            })),
          },
        ],
      }
    : {}

  const handleClick = (params: unknown) => {
    const p = params as { dataIndex?: number }
    if (p.dataIndex !== undefined && partitions[p.dataIndex]) {
      const cat = partitions[p.dataIndex].category
      setSelectedCategory(cat === selectedCategory ? null : cat)
    }
  }

  const extra = (
    <div className="flex gap-1">
      {METRICS.map((m) => (
        <button
          key={m.key}
          onClick={() => setMetric(m.key)}
          className={`px-2 py-0.5 text-[10px] rounded transition-colors ${
            metric === m.key ? 'bg-bili-blue text-white' : 'text-text-muted hover:text-text-secondary'
          }`}
        >
          {m.label}
        </button>
      ))}
    </div>
  )

  return (
    <Card title="分区排行" extra={extra} className="h-full">
      <ChartWrapper option={option} loading={loading} onEvents={{ click: handleClick }} />
    </Card>
  )
}
