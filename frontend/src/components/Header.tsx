import { useApi } from '../hooks/useApi'
import { fetchOverview } from '../api'

export default function Header() {
  const { data } = useApi(fetchOverview)

  return (
    <header className="flex items-center justify-between">
      <div className="flex items-center gap-4">
        <div className="relative">
          <div className="w-1.5 h-8 bg-gradient-to-b from-bili-blue to-bili-pink rounded-full" />
          <div className="absolute inset-0 w-1.5 h-8 bg-gradient-to-b from-bili-blue to-bili-pink rounded-full blur-md opacity-60" />
        </div>
        <div>
          <h1 className="font-display text-2xl tracking-wider text-text-primary leading-none">
            <span className="glow-blue">BILIBILI</span>
            <span className="text-bili-blue ml-2 glow-blue">数据终端</span>
          </h1>
          <p className="text-[10px] text-text-muted tracking-widest mt-0.5 font-mono-num">
            HOT VIDEO ANALYSIS PLATFORM
          </p>
        </div>
      </div>

      {data && (
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-bg-card border border-border-default">
            <div className="w-1.5 h-1.5 rounded-full bg-positive animate-pulse" />
            <span className="text-[10px] text-text-muted">LIVE</span>
          </div>
          <div className="text-right">
            <div className="text-[10px] text-text-muted">更新时间</div>
            <div className="font-mono-num text-xs text-text-secondary">{data.update_time}</div>
          </div>
          <div className="text-right">
            <div className="text-[10px] text-text-muted">快照日期</div>
            <div className="font-mono-num text-sm text-bili-blue font-semibold">{data.latest_snapshot_date ?? '-'}</div>
          </div>
        </div>
      )}
    </header>
  )
}
