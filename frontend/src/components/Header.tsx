import { useApi } from '../hooks/useApi'
import { fetchOverview } from '../api'

export default function Header() {
  const { data } = useApi(fetchOverview)

  return (
    <header className="flex items-center justify-between">
      <div className="flex items-center gap-3">
        <div className="w-2 h-8 bg-bili-blue rounded-full" />
        <h1 className="font-display text-2xl tracking-wider text-text-primary">
          BILIBILI
          <span className="text-bili-blue ml-2">数据终端</span>
        </h1>
      </div>

      {data && (
        <div className="flex items-center gap-6 text-sm">
          <span className="text-text-muted">
            更新时间：
            <span className="font-mono-num text-text-secondary">{data.update_time}</span>
          </span>
          <span className="text-text-muted">
            快照日期：
            <span className="font-mono-num text-bili-blue">{data.latest_snapshot_date ?? '-'}</span>
          </span>
        </div>
      )}
    </header>
  )
}
