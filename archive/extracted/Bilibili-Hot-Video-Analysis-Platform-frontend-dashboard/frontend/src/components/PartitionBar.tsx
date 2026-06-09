import { useApi } from '../hooks/useApi'
import { fetchPartitionCompare } from '../api'
import { useDashboard } from '../context/DashboardContext'
import Card from './common/Card'
import ChartWrapper from './common/ChartWrapper'
import type { EChartsOption } from 'echarts'

export default function PartitionBar() {
  const { data, loading } = useApi(fetchPartitionCompare)
  const { selectedCategory, setSelectedCategory } = useDashboard()

  const option: EChartsOption = data
    ? {
        grid: { left: 60, right: 16, top: 8, bottom: 24 },
        xAxis: {
          type: 'value',
          axisLabel: { formatter: (v: number) => (v >= 10000 ? v / 10000 + 'w' : String(v)) },
        },
        yAxis: {
          type: 'category',
          data: [...data.partitions].reverse().map((p) => p.category),
          axisLabel: { color: '#E6EDF3', fontSize: 11 },
        },
        tooltip: {
          trigger: 'axis',
          axisPointer: { type: 'shadow' },
        },
        series: [
          {
            type: 'bar',
            data: [...data.partitions].reverse().map((p) => ({
              value: p.hot_video_count,
              itemStyle: {
                color: p.category === selectedCategory ? '#FB7299' : '#00A1D6',
                borderRadius: [0, 4, 4, 0],
              },
            })),
            barWidth: '60%',
          },
        ],
      }
    : {}

  const handleClick = (params: unknown) => {
    const p = params as { dataIndex?: number }
    if (data && typeof p.dataIndex === 'number') {
      const reversed = [...data.partitions].reverse()
      const cat = reversed[p.dataIndex]?.category
      if (cat) setSelectedCategory(cat === selectedCategory ? null : cat)
    }
  }

  return (
    <Card title="分区排行" className="h-full">
      <ChartWrapper option={option} loading={loading} onEvents={{ click: handleClick }} />
    </Card>
  )
}
