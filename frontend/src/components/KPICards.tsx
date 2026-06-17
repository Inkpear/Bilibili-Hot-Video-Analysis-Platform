import { useApi } from '../hooks/useApi'
import { fetchOverview } from '../api'
import { formatNumber } from '../utils/format'
import { PlayCircle, MessageCircle, ThumbsUp, Coins, Heart } from 'lucide-react'
import { useDashboard } from '../context/DashboardContext'
import type { LucideIcon } from 'lucide-react'
import type { OverviewData } from '../types'

const KPI_ITEMS: { label: string; key: keyof OverviewData; suffix: string; color: string; icon: LucideIcon }[] = [
  { label: '总播放量', key: 'total_views', suffix: '', color: 'text-bili-blue', icon: PlayCircle },
  { label: '总互动量', key: 'total_interactions', suffix: '', color: 'text-info', icon: MessageCircle },
  { label: '总点赞', key: 'total_likes', suffix: '', color: 'text-bili-pink', icon: ThumbsUp },
  { label: '总投币', key: 'total_coins', suffix: '', color: 'text-warning', icon: Coins },
  { label: '总收藏', key: 'total_favorites', suffix: '', color: 'text-positive', icon: Heart },
]

export default function KPICards() {
  const { selectedSnapshotDate } = useDashboard()
  const { data, loading } = useApi(() => fetchOverview(selectedSnapshotDate), [selectedSnapshotDate])

  return (
    <div className="grid grid-cols-5 gap-3">
      {KPI_ITEMS.map((item, i) => (
        <div
          key={item.key}
          className="card px-4 py-3 flex items-center gap-3 animate-fade-in-up group"
          style={{ animationDelay: `${0.15 + i * 0.08}s` }}
        >
          <div className="flex-shrink-0 w-9 h-9 rounded-lg bg-bg-elevated flex items-center justify-center opacity-60 group-hover:opacity-100 transition-opacity">
            <item.icon size={18} className={item.color} />
          </div>
          <div className="flex flex-col gap-0.5 min-w-0">
            <span className="text-[10px] text-text-muted tracking-wide">{item.label}</span>
            {loading ? (
              <div className="animate-shimmer rounded h-6 w-20" />
            ) : (
              <span className={`font-mono-num text-lg font-bold ${item.color} leading-none`}>
                {data ? formatNumber(data[item.key] as number ?? 0) : '-'}
                {item.suffix && <span className="text-[10px] text-text-muted ml-1 font-normal">{item.suffix}</span>}
              </span>
            )}
          </div>
        </div>
      ))}
    </div>
  )
}
