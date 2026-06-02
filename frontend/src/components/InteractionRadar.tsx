import { useApi } from '../hooks/useApi'
import { fetchInteraction } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatPercent } from '../utils/format'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import type { EChartsOption } from 'echarts'

export default function InteractionRadar() {
  const { selectedBvid } = useDashboard()
  const { data, loading } = useApi(
    () => selectedBvid ? fetchInteraction(selectedBvid) : Promise.resolve(null),
    [selectedBvid]
  )

  const option: EChartsOption = data
    ? {
        radar: {
          indicator: [
            { name: '点赞率', max: 0.2 },
            { name: '投币率', max: 0.15 },
            { name: '收藏率', max: 0.2 },
            { name: '回复率', max: 0.05 },
            { name: '分享率', max: 0.03 },
          ],
          axisName: { color: '#8B949E', fontSize: 11 },
          splitArea: { areaStyle: { color: ['rgba(0,161,214,0.02)', 'rgba(0,161,214,0.04)'] } },
          splitLine: { lineStyle: { color: '#21262D' } },
          axisLine: { lineStyle: { color: '#30363D' } },
        },
        tooltip: {
          trigger: 'item',
          formatter: (params: unknown) => {
            const p = params as { value?: number[] }
            if (!p.value) return ''
            const labels = ['点赞率', '投币率', '收藏率', '回复率', '分享率']
            return p.value.map((v, i) => `${labels[i]}: ${formatPercent(v)}`).join('<br/>')
          },
        },
        series: [
          {
            type: 'radar',
            data: [
              {
                value: [data.like_rate, data.coin_rate, data.fav_rate, data.reply_rate, data.share_rate],
                areaStyle: { color: 'rgba(0, 161, 214, 0.2)' },
                lineStyle: { color: '#00A1D6', width: 2 },
                itemStyle: { color: '#00A1D6' },
              },
            ],
          },
        ],
      }
    : {}

  return (
    <Card title="互动质量" className="h-full">
      {!selectedBvid ? (
        <div className="flex items-center justify-center h-full text-text-muted text-sm">
          ← 选择一个视频查看互动分析
        </div>
      ) : (
        <div className="flex flex-col h-full">
          <div className="flex-1 min-h-0">
            <ChartWrapper option={option} loading={loading} />
          </div>
          {data && (
            <div className="grid grid-cols-5 gap-1 pt-2 border-t border-border-default">
              {[
                { label: '点赞率', value: data.like_rate },
                { label: '投币率', value: data.coin_rate },
                { label: '收藏率', value: data.fav_rate },
                { label: '回复率', value: data.reply_rate },
                { label: '分享率', value: data.share_rate },
              ].map((item) => (
                <div key={item.label} className="text-center">
                  <div className="text-xs text-text-muted">{item.label}</div>
                  <div className="font-mono-num text-sm text-bili-blue">{formatPercent(item.value)}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
