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
const COEFFICIENT = 5

export default function InteractionRadar() {
  const { selectedBvid } = useDashboard()

  const singleData = useApi(
    () => selectedBvid ? fetchInteraction(selectedBvid) : Promise.resolve(null),
    [selectedBvid]
  )

  const maxData = useApi(() => fetchInteractionMax(), [])

  const loading = selectedBvid ? singleData.loading : maxData.loading

  const rates = useMemo(() => {
    if (selectedBvid && singleData.data) {
      return [singleData.data.like_rate, singleData.data.coin_rate, singleData.data.fav_rate, singleData.data.reply_rate, singleData.data.share_rate]
    }
    if (maxData.data) {
      return [maxData.data.max_like_rate, maxData.data.max_coin_rate, maxData.data.max_fav_rate, maxData.data.max_reply_rate, maxData.data.max_share_rate]
    }
    return null
  }, [selectedBvid, singleData.data, maxData.data])

  const maxRates = useMemo(() => {
    if (!maxData.data) return null
    return [maxData.data.max_like_rate, maxData.data.max_coin_rate, maxData.data.max_fav_rate, maxData.data.max_reply_rate, maxData.data.max_share_rate]
  }, [maxData.data])

  const option: EChartsOption = useMemo(() => {
    if (!rates) return {}

    const isOverview = !selectedBvid
    const scaledValues = rates.map((v) => v * COEFFICIENT)

    const indicator = rates.map((v, i) => {
      if (isOverview) {
        return { name: LABELS[i], max: v * COEFFICIENT }
      }
      const m = maxRates ? maxRates[i] : v
      return { name: LABELS[i], max: Math.max(m * COEFFICIENT * 1.2, 0.5) }
    })

    return {
      radar: {
        indicator,
        axisName: { color: '#8B949E', fontSize: 11 },
        splitArea: { areaStyle: { color: ['rgba(0,161,214,0.02)', 'rgba(0,161,214,0.04)'] } },
        splitLine: { lineStyle: { color: '#21262D' } },
        axisLine: { lineStyle: { color: '#30363D' } },
        radius: '65%',
      },
      tooltip: {
        trigger: 'item',
        formatter: () => {
          const prefix = isOverview ? '样本最大 ' : ''
          return rates.map((v, i) => `${prefix}${LABELS[i]}: ${formatPercent(v)}`).join('<br/>')
        },
      },
      series: [
        {
          type: 'radar',
          data: [
            {
              value: scaledValues,
              areaStyle: {
                color: isOverview ? 'rgba(251, 114, 153, 0.25)' : 'rgba(0, 161, 214, 0.25)',
              },
              lineStyle: {
                color: isOverview ? '#FB7299' : '#00A1D6',
                width: 2,
              },
              itemStyle: { color: isOverview ? '#FB7299' : '#00A1D6' },
              symbol: 'circle',
              symbolSize: 6,
            },
          ],
        },
      ],
    } as EChartsOption
  }, [rates, maxRates, selectedBvid])

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
              样本各指标最大值（选择视频查看具体数据）
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
