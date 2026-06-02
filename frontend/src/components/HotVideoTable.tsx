import { useState } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchHotVideos } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'
import type { HotVideosSort } from '../types'

const SORT_OPTIONS: { value: HotVideosSort; label: string }[] = [
  { value: 'heat', label: '热度' },
  { value: 'views', label: '播放' },
  { value: 'likes', label: '点赞' },
  { value: 'coins', label: '投币' },
]

export default function HotVideoTable() {
  const [sort, setSort] = useState<HotVideosSort>('heat')
  const [offset, setOffset] = useState(0)
  const limit = 12

  const { data, loading } = useApi(() => fetchHotVideos(sort, limit, offset), [sort, offset])
  const { selectedBvid, setSelectedBvid, selectedCategory } = useDashboard()

  const videos = data?.videos.filter(
    (v) => !selectedCategory || v.category === selectedCategory
  ) ?? []

  const extra = (
    <div className="flex gap-1">
      {SORT_OPTIONS.map((s) => (
        <button
          key={s.value}
          onClick={() => { setSort(s.value); setOffset(0); }}
          className={`px-2 py-0.5 text-xs rounded transition-colors ${
            sort === s.value
              ? 'bg-bili-blue text-white'
              : 'text-text-muted hover:text-text-secondary'
          }`}
        >
          {s.label}
        </button>
      ))}
    </div>
  )

  return (
    <Card title="热门视频" extra={extra} className="h-full">
      {loading ? (
        <Loading rows={8} />
      ) : (
        <div className="flex flex-col h-full">
          <div className="flex-1 overflow-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-text-muted border-b border-border-default">
                  <th className="py-1.5 text-left w-8">#</th>
                  <th className="py-1.5 text-left">标题</th>
                  <th className="py-1.5 text-right">播放</th>
                  <th className="py-1.5 text-right">点赞</th>
                  <th className="py-1.5 text-right">热度</th>
                </tr>
              </thead>
              <tbody>
                {videos.map((v) => (
                  <tr
                    key={v.bvid}
                    onClick={() => setSelectedBvid(v.bvid === selectedBvid ? null : v.bvid)}
                    className={`cursor-pointer border-b border-border-default/50 transition-colors ${
                      v.bvid === selectedBvid
                        ? 'bg-bili-blue/10 border-l-2 border-l-bili-blue'
                        : 'hover:bg-bg-elevated'
                    }`}
                  >
                    <td className="py-1.5 font-mono-num text-text-muted">{v.rank_position}</td>
                    <td className="py-1.5 max-w-[200px] truncate text-text-primary">{v.title}</td>
                    <td className="py-1.5 text-right font-mono-num text-text-secondary">{formatNumber(v.view_count)}</td>
                    <td className="py-1.5 text-right font-mono-num text-text-secondary">{formatNumber(v.like_count)}</td>
                    <td className="py-1.5 text-right font-mono-num text-bili-pink">{v.heat_score.toFixed(3)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="flex items-center justify-between pt-2 border-t border-border-default mt-auto">
            <span className="text-xs text-text-muted">共 {data?.total ?? 0} 条</span>
            <div className="flex gap-1">
              <button
                onClick={() => setOffset(Math.max(0, offset - limit))}
                disabled={offset === 0}
                className="px-2 py-0.5 text-xs rounded bg-bg-elevated text-text-secondary disabled:opacity-30 hover:bg-border-default transition-colors"
              >
                上一页
              </button>
              <button
                onClick={() => setOffset(offset + limit)}
                disabled={!data || offset + limit >= data.total}
                className="px-2 py-0.5 text-xs rounded bg-bg-elevated text-text-secondary disabled:opacity-30 hover:bg-border-default transition-colors"
              >
                下一页
              </button>
            </div>
          </div>
        </div>
      )}
    </Card>
  )
}
