import { useApi } from '../hooks/useApi'
import { fetchOverview } from '../api'
import { formatNumber } from '../utils/format'

const KPI_ITEMS = [
  { label: '热门视频', key: 'video_count' as const, suffix: '个', color: 'text-bili-blue', icon: '📊' },
  { label: '覆盖分区', key: 'category_count' as const, suffix: '个', color: 'text-bili-pink', icon: '📁' },
  { label: '独立UP主', key: 'up_count' as const, suffix: '人', color: 'text-info', icon: '👤' },
  { label: '总播放量', key: 'total_views' as const, suffix: '', color: 'text-positive', icon: '▶' },
  { label: '总互动量', key: 'total_interactions' as const, suffix: '', color: 'text-warning', icon: '💬' },
]

export default function KPICards() {
  const { data, loading } = useApi(fetchOverview)

  return (
    <div className="grid grid-cols-5 gap-3">
      {KPI_ITEMS.map((item, i) => (
        <div
          key={item.key}
          className="card px-4 py-3 flex items-center gap-3 animate-fade-in-up group"
          style={{ animationDelay: `${0.15 + i * 0.08}s` }}
        >
          <div className="flex-shrink-0 w-9 h-9 rounded-lg bg-bg-elevated flex items-center justify-center text-lg opacity-60 group-hover:opacity-100 transition-opacity">
            {item.icon}
          </div>
          <div className="flex flex-col gap-0.5 min-w-0">
            <span className="text-[10px] text-text-muted tracking-wide">{item.label}</span>
            {loading ? (
              <div className="animate-shimmer rounded h-6 w-20" />
            ) : (
              <span className={`font-mono-num text-lg font-bold ${item.color} leading-none`}>
                {data ? formatNumber(data[item.key]) : '-'}
                {item.suffix && <span className="text-[10px] text-text-muted ml-1 font-normal">{item.suffix}</span>}
              </span>
            )}
          </div>
        </div>
      ))}
    </div>
  )
}
