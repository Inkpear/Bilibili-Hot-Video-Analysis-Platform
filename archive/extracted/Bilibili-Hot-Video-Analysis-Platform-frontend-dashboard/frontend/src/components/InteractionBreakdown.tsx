import { useMemo } from 'react'
import type { EChartsOption } from 'echarts'
import { fetchHotVideos, fetchInteraction } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { useApi } from '../hooks/useApi'
import { formatNumber, formatPercent } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'

const METRICS = [
  { key: 'like_count', rateKey: 'like_rate', label: '点赞', color: '#00A1D6' },
  { key: 'coin_count', rateKey: 'coin_rate', label: '投币', color: '#79C0FF' },
  { key: 'favorite_count', rateKey: 'fav_rate', label: '收藏', color: '#FB7299' },
  { key: 'reply_count', rateKey: 'reply_rate', label: '评论', color: '#F0883E' },
  { key: 'share_count', rateKey: 'share_rate', label: '分享', color: '#39D353' },
] as const

export default function InteractionBreakdown() {
  const { selectedBvid, selectedCategory, sort } = useDashboard()

  const leaderData = useApi(
    () => (selectedBvid ? Promise.resolve(null) : fetchHotVideos(sort, 1, 0, selectedCategory ?? undefined)),
    [selectedBvid, selectedCategory, sort]
  )

  const focusBvid = selectedBvid ?? leaderData.data?.videos[0]?.bvid ?? null
  const focusTitle = selectedBvid ? selectedBvid : leaderData.data?.videos[0]?.title ?? '等待视频数据'

  const interactionData = useApi(() => (focusBvid ? fetchInteraction(focusBvid) : Promise.resolve(null)), [focusBvid])
  const detail = interactionData.data

  const option = useMemo<EChartsOption>(() => {
    if (!detail) return {}

    return {
      animationDuration: 360,
      grid: {
        left: 46,
        right: 20,
        top: 4,
        bottom: 10,
      },
      tooltip: {
        trigger: 'axis',
        axisPointer: { type: 'shadow' },
        formatter: (params: unknown) => {
          const items = Array.isArray(params) ? params : [params]
          const first = items[0] as { dataIndex: number }
          const metric = METRICS[first.dataIndex]
          if (!metric) return ''

          const count = detail[metric.key]
          const rate = detail[metric.rateKey]

          return [
            `<div style="font-size:12px;">`,
            `<div style="color:#E6EDF3;font-weight:600;margin-bottom:4px;">${metric.label}</div>`,
            `<div>互动量: <span style="color:${metric.color};font-family:JetBrains Mono, monospace;">${formatNumber(count)}</span></div>`,
            `<div>转化率: <span style="color:#8B949E;font-family:JetBrains Mono, monospace;">${formatPercent(rate)}</span></div>`,
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
        data: METRICS.map((metric) => metric.label),
        axisTick: { show: false },
        axisLine: { show: false },
        axisLabel: {
          color: '#E6EDF3',
          fontSize: 10,
        },
      },
      series: [
        {
          type: 'bar',
          barWidth: 14,
          data: METRICS.map((metric) => ({
            value: detail[metric.key],
            itemStyle: {
              color: metric.color,
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
  }, [detail])

  const totalInteractions = detail
    ? detail.like_count + detail.coin_count + detail.favorite_count + detail.reply_count + detail.share_count
    : 0

  return (
    <Card
      title="互动构成"
      extra={<span className="text-[10px] text-text-muted">{selectedBvid ? '当前选中视频' : '当前榜单 Top 1'}</span>}
      className="h-full"
    >
      <div className="flex h-full min-h-0 flex-col gap-2">
        <div className="space-y-2">
          <div className="truncate text-sm font-semibold text-text-primary">{focusTitle}</div>

          <div className="grid grid-cols-2 gap-2">
            <div className="rounded-lg border border-border-default bg-bg-elevated/55 px-2.5 py-2">
              <div className="text-[10px] text-text-muted">播放基数</div>
              <div className="mt-1 font-mono-num text-base font-semibold text-bili-blue">
                {detail ? formatNumber(detail.view_count) : '-'}
              </div>
            </div>
            <div className="rounded-lg border border-border-default bg-bg-elevated/55 px-2.5 py-2">
              <div className="text-[10px] text-text-muted">总互动量</div>
              <div className="mt-1 font-mono-num text-base font-semibold text-bili-pink">
                {detail ? formatNumber(totalInteractions) : '-'}
              </div>
            </div>
          </div>
        </div>

        {detail ? (
          <>
            <div className="flex-1 min-h-0">
              <ChartWrapper option={option} loading={leaderData.loading || interactionData.loading} />
            </div>

            <div className="grid grid-cols-5 gap-1.5 border-t border-border-default pt-2">
              {METRICS.map((metric) => (
                <div key={metric.key} className="rounded bg-bg-elevated/35 px-1.5 py-1.5 text-center">
                  <div className="text-[9px] text-text-muted">{metric.label}率</div>
                  <div className="mt-1 font-mono-num text-[11px] font-semibold" style={{ color: metric.color }}>
                    {formatPercent(detail[metric.rateKey])}
                  </div>
                </div>
              ))}
            </div>
          </>
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-text-muted">暂无互动结构数据</div>
        )}
      </div>
    </Card>
  )
}
