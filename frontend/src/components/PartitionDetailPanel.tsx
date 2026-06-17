import { useMemo } from 'react'
import type { EChartsOption } from 'echarts'
import { fetchPartitionCompare, fetchPartitionDetail } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { useApi } from '../hooks/useApi'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'

const BAR_COLORS = {
  video: '#00A1D6',
  up: '#FB7299',
}

function ellipsis(value: string, max = 12) {
  return value.length > max ? `${value.slice(0, max)}…` : value
}

export default function PartitionDetailPanel() {
  const { selectedCategory, selectedSnapshotDate } = useDashboard()
  const compareData = useApi(() => fetchPartitionCompare(selectedSnapshotDate), [selectedSnapshotDate])

  const fallbackCategory = useMemo(() => {
    const partitions = compareData.data?.partitions ?? []
    if (partitions.length === 0) return null
    return [...partitions].sort((a, b) => b.total_views - a.total_views)[0]?.category ?? null
  }, [compareData.data])

  const focusCategory = selectedCategory ?? fallbackCategory
  const detailData = useApi(
    () => (focusCategory ? fetchPartitionDetail(focusCategory, selectedSnapshotDate) : Promise.resolve(null)),
    [focusCategory, selectedSnapshotDate]
  )

  const videoOption = useMemo<EChartsOption>(() => {
    const detail = detailData.data
    if (!detail) return {}

    const videos = [...detail.videos].sort((a, b) => b.view_count - a.view_count).slice(0, 5)

    return {
      animationDuration: 360,
      grid: {
        left: 104,
        right: 52,
        top: 6,
        bottom: 24,
      },
      tooltip: {
        trigger: 'axis',
        axisPointer: { type: 'shadow' },
        formatter: (params: unknown) => {
          const items = Array.isArray(params) ? params : [params]
          const first = items[0] as { dataIndex: number }
          const current = videos[first.dataIndex]
          if (!current) return ''

          return [
            `<div style="font-size:12px;">`,
            `<div style="color:#E6EDF3;font-weight:600;margin-bottom:4px;">${current.title}</div>`,
            `<div>播放量: <span style="color:#79C0FF;font-family:JetBrains Mono, monospace;">${formatNumber(current.view_count)}</span></div>`,
            `<div>热度值: <span style="color:#FB7299;font-family:JetBrains Mono, monospace;">${current.heat_score.toFixed(3)}</span></div>`,
            `<div>榜单名次: <span style="color:#8B949E;font-family:JetBrains Mono, monospace;">#${current.rank_position}</span></div>`,
            `</div>`,
          ].join('')
        },
      },
      xAxis: {
        type: 'value',
        axisLabel: {
          color: '#8B949E',
          fontSize: 9,
          formatter: (value: number) => formatNumber(value),
        },
        splitLine: { lineStyle: { color: '#21262D', type: 'dashed' } },
      },
      yAxis: {
        type: 'category',
        inverse: true,
        data: videos.map((video) => ellipsis(video.title, 10)),
        axisTick: { show: false },
        axisLine: { show: false },
        axisLabel: {
          color: '#E6EDF3',
          fontSize: 9,
          width: 92,
          overflow: 'truncate',
        },
      },
      series: [
        {
          type: 'bar',
          barWidth: 14,
          data: videos.map((video) => ({
            value: video.view_count,
            itemStyle: {
              color: BAR_COLORS.video,
              borderRadius: [0, 6, 6, 0],
            },
          })),
          label: {
            show: true,
            position: 'right',
            color: '#E6EDF3',
            fontSize: 9,
            formatter: (params: unknown) => formatNumber(Number((params as { value: number }).value)),
          },
        },
      ],
    }
  }, [detailData.data])

  const upOption = useMemo<EChartsOption>(() => {
    const detail = detailData.data
    if (!detail) return {}

    const ups = [...detail.top_ups].sort((a, b) => b.total_views - a.total_views).slice(0, 5)

    return {
      animationDuration: 360,
      grid: {
        left: 84,
        right: 52,
        top: 6,
        bottom: 24,
      },
      tooltip: {
        trigger: 'axis',
        axisPointer: { type: 'shadow' },
        formatter: (params: unknown) => {
          const items = Array.isArray(params) ? params : [params]
          const first = items[0] as { dataIndex: number }
          const current = ups[first.dataIndex]
          if (!current) return ''

          return [
            `<div style="font-size:12px;">`,
            `<div style="color:#E6EDF3;font-weight:600;margin-bottom:4px;">${current.up_name}</div>`,
            `<div>总播放: <span style="color:#FB7299;font-family:JetBrains Mono, monospace;">${formatNumber(current.total_views)}</span></div>`,
            `<div>粉丝数: <span style="color:#79C0FF;font-family:JetBrains Mono, monospace;">${formatNumber(current.follower_count)}</span></div>`,
            `<div>上榜视频: <span style="color:#8B949E;font-family:JetBrains Mono, monospace;">${current.hot_video_count} 条</span></div>`,
            `</div>`,
          ].join('')
        },
      },
      xAxis: {
        type: 'value',
        axisLabel: {
          color: '#8B949E',
          fontSize: 9,
          formatter: (value: number) => formatNumber(value),
        },
        splitLine: { lineStyle: { color: '#21262D', type: 'dashed' } },
      },
      yAxis: {
        type: 'category',
        inverse: true,
        data: ups.map((up) => ellipsis(up.up_name, 8)),
        axisTick: { show: false },
        axisLine: { show: false },
        axisLabel: {
          color: '#E6EDF3',
          fontSize: 9,
          width: 72,
          overflow: 'truncate',
        },
      },
      series: [
        {
          type: 'bar',
          barWidth: 14,
          data: ups.map((up) => ({
            value: up.total_views,
            itemStyle: {
              color: BAR_COLORS.up,
              borderRadius: [0, 6, 6, 0],
            },
          })),
          label: {
            show: true,
            position: 'right',
            color: '#E6EDF3',
            fontSize: 9,
            formatter: (params: unknown) => formatNumber(Number((params as { value: number }).value)),
          },
        },
      ],
    }
  }, [detailData.data])

  const detail = detailData.data
  const modeLabel = selectedCategory ? '当前筛选' : '自动聚焦'

  return (
    <Card
      title="分区下钻"
      extra={
        <div className="flex items-center gap-2">
          <span className="rounded-full bg-bg-elevated px-2 py-0.5 text-[10px] text-text-muted">{modeLabel}</span>
          <span className="rounded-full bg-bili-blue/15 px-2 py-0.5 text-[10px] text-bili-blue">
            {focusCategory ?? '等待数据'}
          </span>
        </div>
      }
      className="h-full"
    >
      <div className="flex h-full min-h-0 flex-col gap-2">
        {detail ? (
          <>
            <div className="flex flex-wrap items-center gap-3 text-[10px] text-text-muted">
              <span>样本 {detail.hot_video_count}</span>
              <span>平均播放 {formatNumber(detail.avg_views)}</span>
              <span>左看视频，右看 UP 主</span>
            </div>

            <div className="grid flex-1 min-h-0 grid-cols-1 gap-3 md:grid-cols-[1.15fr_0.85fr]">
              <div className="flex min-h-0 flex-col gap-1.5 overflow-hidden">
                <div className="flex items-center justify-between shrink-0">
                  <h4 className="text-[11px] font-semibold text-text-secondary">分区热视频 Top 5</h4>
                  <span className="text-[10px] text-text-muted">按播放量排序</span>
                </div>
                <div className="flex-1 min-h-0">
                  <ChartWrapper option={videoOption} loading={detailData.loading} />
                </div>
              </div>

              <div className="flex min-h-0 flex-col gap-1.5 overflow-hidden">
                <div className="flex items-center justify-between shrink-0">
                  <h4 className="text-[11px] font-semibold text-text-secondary">头部 UP 贡献</h4>
                  <span className="text-[10px] text-text-muted">按总播放排序</span>
                </div>
                <div className="flex-1 min-h-0">
                  <ChartWrapper option={upOption} loading={detailData.loading} />
                </div>
              </div>
            </div>
          </>
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-text-muted">暂无分区下钻数据</div>
        )}
      </div>
    </Card>
  )
}
