import { useMemo } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchInteraction } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatPercent } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import type { EChartsOption } from 'echarts'

const LABELS = ['点赞率', '投币率', '收藏率', '回复率', '分享率']
const COEFFICIENT = 5

export default function InteractionRadar() {
  const { selectedBvid } = useDashboard()
  const { data, loading } = useApi(
    () => selectedBvid ? fetchInteraction(selectedBvid) : Promise.resolve(null),
    [selectedBvid]
  )

  const option: EChartsOption = useMemo(() => {
    if (!data) return {}

    const rates = [data.like_rate, data.coin_rate, data.fav_rate, data.reply_rate, data.share_rate]
    const scaledValues = rates.map((v) => v * COEFFICIENT)
    const indicator = rates.map((v, i) => ({
      name: LABELS[i],
      max: Math.max(Math.ceil(v * COEFFICIENT * 1.5 * 100) / 100, 0.5),
    }))

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
          return rates.map((v, i) => `${LABELS[i]}: ${formatPercent(v)}`).join('<br/>')
        },
      },
      series: [
        {
          type: 'radar',
          data: [
            {
              value: scaledValues,
              areaStyle: { color: 'rgba(0, 161, 214, 0.25)' },
              lineStyle: { color: '#00A1D6', width: 2 },
              itemStyle: { color: '#00A1D6' },
              symbol: 'circle',
              symbolSize: 6,
            },
          ],
        },
      ],
    } as EChartsOption
  }, [data])

  return (
    <Card title="互动质量" className="h-full">
      {!selectedBvid ? (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          ← 选择一个视频查看互动分析
        </div>
      ) : (
        <div className="flex flex-col h-full min-h-0">
          <div className="flex-1 min-h-0">
            <ChartWrapper option={option} loading={loading} />
          </div>
          {data && (
            <div className="grid grid-cols-5 gap-1 pt-2 border-t border-border-default shrink-0">
              {[
                { label: '点赞率', value: data.like_rate, color: '#00A1D6' },
                { label: '投币率', value: data.coin_rate, color: '#FB7299' },
                { label: '收藏率', value: data.fav_rate, color: '#39D353' },
                { label: '回复率', value: data.reply_rate, color: '#F0883E' },
                { label: '分享率', value: data.share_rate, color: '#A371F7' },
              ].map((item) => (
                <div key={item.label} className="text-center">
                  <div className="text-[10px] text-text-muted">{item.label}</div>
                  <div className="font-mono-num text-sm font-semibold" style={{ color: item.color }}>
                    {formatPercent(item.value)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
