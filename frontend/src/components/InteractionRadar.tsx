import { useMemo } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchInteraction, fetchInteractionMax } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatPercent } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import Loading from './common/Loading'
import type { EChartsOption } from 'echarts'

const LABELS = ['点赞率', '投币率', '收藏率', '回复率', '分享率']

// 独立极值：各维度"优秀爆款"标准（基于 B站数据分布）
const EXCELLENT_MAX = [0.12, 0.10, 0.05, 0.015, 0.02]

// 视觉保底：映射到 [BASE_RATIO, 1.0] 区间，即使 0% 也有底座
const BASE_RATIO = 0.15

function mapValue(raw: number, max: number): number {
  const clamped = Math.max(0, Math.min(raw, max))
  return BASE_RATIO + (1 - BASE_RATIO) * (clamped / max)
}

export default function InteractionRadar() {
  const { selectedBvid, selectedSnapshotDate } = useDashboard()

  const singleData = useApi(
    () => selectedBvid ? fetchInteraction(selectedBvid, selectedSnapshotDate) : Promise.resolve(null),
    [selectedBvid, selectedSnapshotDate]
  )

  const maxData = useApi(() => fetchInteractionMax(selectedSnapshotDate), [selectedSnapshotDate])

  const loading = selectedBvid ? singleData.loading : maxData.loading

  const rates = useMemo(() => {
    if (selectedBvid && singleData.data) {
      return [
        singleData.data.like_rate,
        singleData.data.coin_rate,
        singleData.data.fav_rate,
        singleData.data.reply_rate,
        singleData.data.share_rate,
      ]
    }
    if (maxData.data) {
      return [
        maxData.data.max_like_rate,
        maxData.data.max_coin_rate,
        maxData.data.max_fav_rate,
        maxData.data.max_reply_rate,
        maxData.data.max_share_rate,
      ]
    }
    return null
  }, [selectedBvid, singleData.data, maxData.data])

  const option: EChartsOption = useMemo(() => {
    if (!rates) return {}

    const isOverview = !selectedBvid

    // 映射到 [BASE_RATIO, 1.0] 区间
    const mappedValues = rates.map((v, i) => mapValue(v, EXCELLENT_MAX[i]))

    // indicator max 固定为 1.0（因为值已被归一化到 0~1）
    const indicator = LABELS.map((name) => ({ name, max: 1.0 }))

    const color = isOverview ? '#FB7299' : '#00A1D6'
    const fill = isOverview ? 'rgba(251, 114, 153, 0.2)' : 'rgba(0, 161, 214, 0.2)'

    return {
      radar: {
        indicator,
        center: ['50%', '52%'],
        radius: '68%',
        startAngle: 90,
        shape: 'polygon',
        axisName: {
          color: '#8B949E',
          fontSize: 11,
          fontFamily: 'Noto Sans SC, sans-serif',
        },
        splitNumber: 4,
        splitArea: {
          areaStyle: {
            color: [
              'rgba(0, 161, 214, 0.01)',
              'rgba(0, 161, 214, 0.03)',
              'rgba(0, 161, 214, 0.01)',
              'rgba(0, 161, 214, 0.03)',
            ],
          },
        },
        splitLine: { lineStyle: { color: '#3D444D' } },
        axisLine: { lineStyle: { color: '#3D444D' } },
      },
      tooltip: {
        trigger: 'item',
        backgroundColor: '#1C2129',
        borderColor: 'rgba(0,161,214,0.2)',
        textStyle: { color: '#E6EDF3', fontSize: 12 },
        formatter: () => {
          const prefix = isOverview ? '样本最大' : '当前视频'
          const lines = rates.map((v, i) => {
            const pct = Math.min(v / EXCELLENT_MAX[i], 1) * 100
            const color = isOverview ? '#FB7299' : '#00A1D6'
            return `<div style="display:flex;align-items:center;gap:8px;margin:3px 0">
              <span style="width:36px;text-align:right;font-size:11px;color:#8B949E">${LABELS[i]}</span>
              <span style="font-family:JetBrains Mono,monospace;font-size:11px;width:42px;text-align:right">${formatPercent(v)}</span>
              <span style="flex:1;height:6px;background:#21262D;border-radius:3px;overflow:hidden">
                <span style="display:block;height:100%;width:${pct}%;background:${color};border-radius:3px"></span>
              </span>
            </div>`
          })
          return `<div style="font-size:12px;padding:2px"><b style="color:#E6EDF3">${prefix}</b>${lines.join('')}</div>`
        },
      },
      series: [
        {
          type: 'radar',
          symbol: 'circle',
          symbolSize: 5,
          data: [
            {
              value: mappedValues,
              areaStyle: { color: fill },
              lineStyle: { color, width: 2 },
              itemStyle: { color, borderColor: '#0D1117', borderWidth: 1 },
            },
          ],
        },
      ],
    } as EChartsOption
  }, [rates, selectedBvid])

  const colors = selectedBvid
    ? ['#00A1D6', '#FB7299', '#39D353', '#F0883E', '#A371F7']
    : ['#FB7299', '#00A1D6', '#39D353', '#F0883E', '#A371F7']

  return (
    <Card title="互动质量" className="h-full">
      {loading ? (
        <Loading rows={4} />
      ) : rates ? (
        <div key={selectedBvid ?? 'max'} className="flex flex-col h-full min-h-0 animate-content-switch">
          {!selectedBvid && (
            <div className="text-[10px] text-text-muted text-center pb-1 shrink-0">
              样本各指标最大值 · 独立极值归一化
            </div>
          )}
          <div className="flex-1 min-h-0">
            <ChartWrapper option={option} />
          </div>
          <div className="grid grid-cols-5 gap-1 pt-2 border-t border-border-default shrink-0">
            {rates.map((v, i) => (
              <div key={LABELS[i]} className="text-center">
                <div className="text-[10px] text-text-muted">{LABELS[i]}</div>
                <div className="font-mono-num text-sm font-semibold" style={{ color: colors[i] }}>
                  {formatPercent(v)}
                </div>
                <div className="text-[9px] text-text-muted mt-0.5">
                  / {formatPercent(EXCELLENT_MAX[i])}
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          暂无数据
        </div>
      )}
    </Card>
  )
}
