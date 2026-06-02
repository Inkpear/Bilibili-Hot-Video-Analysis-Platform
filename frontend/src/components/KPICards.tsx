import { useApi } from '../hooks/useApi'
import { fetchOverview } from '../api'
import { formatNumber } from '../utils/format'

const KPI_ITEMS = [
  { label: '热门视频', key: 'video_count' as const, suffix: '个', color: 'text-bili-blue' },
  { label: '覆盖分区', key: 'category_count' as const, suffix: '个', color: 'text-bili-pink' },
  { label: '独立UP主', key: 'up_count' as const, suffix: '人', color: 'text-info' },
  { label: '总播放量', key: 'total_views' as const, suffix: '', color: 'text-positive' },
  { label: '总互动量', key: 'total_interactions' as const, suffix: '', color: 'text-warning' },
]

export default function KPICards() {
  const { data, loading } = useApi(fetchOverview)

  return (
    <div className="grid grid-cols-5 gap-3">
      {KPI_ITEMS.map((item, i) => (
        <div
          key={item.key}
          className="card px-4 py-3 flex flex-col gap-1 animate-fade-in-up"
          style={{ animationDelay: `${i * 0.08}s` }}
        >
          <span className="text-xs text-text-muted tracking-wide">{item.label}</span>
          {loading ? (
            <div className="animate-shimmer rounded h-7 w-3/4" />
          ) : (
            <span className={`font-mono-num text-xl font-bold ${item.color}`}>
              {data ? formatNumber(data[item.key]) : '-'}
              {item.suffix && <span className="text-xs text-text-muted ml-1">{item.suffix}</span>}
            </span>
          )}
        </div>
      ))}
    </div>
  )
}
