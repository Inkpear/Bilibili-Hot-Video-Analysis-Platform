import { useState, useMemo } from 'react'
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
  const { selectedBvid, setSelectedBvid, selectedCategory, selectedWord } = useDashboard()

  const videos = useMemo(() => {
    if (!data) return []
    let list = data.videos
    if (selectedCategory) {
      list = list.filter((v) => v.category === selectedCategory)
    }
    if (selectedWord) {
      list = list.filter((v) => v.title.includes(selectedWord))
    }
    return list
  }, [data, selectedCategory, selectedWord])

  const total = data?.total ?? 0

  const sortLabel = SORT_OPTIONS.find((s) => s.value === sort)?.label ?? '热度'

  const titleExtra = selectedWord ? (
    <span className="text-bili-pink text-[10px] ml-2">「{selectedWord}」</span>
  ) : null

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
      {titleExtra && (
        <div className="px-4 pt-2 pb-0 text-xs text-text-muted">
          标题包含{titleExtra}的视频
        </div>
      )}
      {loading ? (
        <Loading rows={8} />
      ) : (
        <div className="flex flex-col h-full min-h-0">
          <div className="flex-1 overflow-auto min-h-0">
            <table className="w-full text-xs">
              <thead className="sticky top-0 bg-bg-card z-10">
                <tr className="text-text-muted border-b border-border-default">
                  <th className="py-1.5 text-left w-8">#</th>
                  <th className="py-1.5 text-left">标题</th>
                  <th className="py-1.5 text-left">分区</th>
                  <th className="py-1.5 text-right">播放</th>
                  <th className="py-1.5 text-right">点赞</th>
                  <th className="py-1.5 text-right">{sortLabel}</th>
                </tr>
              </thead>
              <tbody>
                {videos.map((v, i) => (
                  <tr
                    key={v.bvid}
                    onClick={() => setSelectedBvid(v.bvid === selectedBvid ? null : v.bvid)}
                    className={`cursor-pointer border-b border-border-default/50 transition-all duration-150 ${
                      v.bvid === selectedBvid
                        ? 'bg-bili-blue/10 border-l-2 border-l-bili-blue'
                        : 'hover:bg-bg-elevated'
                    }`}
                  >
                    <td className="py-1.5 font-mono-num text-text-muted">{offset + i + 1}</td>
                    <td className="py-1.5 max-w-[200px] truncate text-text-primary">{v.title}</td>
                    <td className="py-1.5 text-text-muted">
                      <span className="px-1.5 py-0.5 rounded bg-bg-elevated text-[10px]">{v.category}</span>
                    </td>
                    <td className="py-1.5 text-right font-mono-num text-text-secondary">{formatNumber(v.view_count)}</td>
                    <td className="py-1.5 text-right font-mono-num text-text-secondary">{formatNumber(v.like_count)}</td>
                    <td className="py-1.5 text-right font-mono-num text-bili-pink font-semibold">
                      {sort === 'heat' ? v.heat_score.toFixed(3) : formatNumber(
                        sort === 'views' ? v.view_count :
                        sort === 'likes' ? v.like_count :
                        v.coin_count
                      )}
                    </td>
                  </tr>
                ))}
                {videos.length === 0 && (
                  <tr>
                    <td colSpan={6} className="py-8 text-center text-text-muted">当前分区无数据</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          <div className="flex items-center justify-between pt-2 border-t border-border-default shrink-0">
            <span className="text-xs text-text-muted">共 {total} 条</span>
            <div className="flex items-center gap-1">
              <button
                onClick={() => setOffset(Math.max(0, offset - limit))}
                disabled={offset === 0}
                className="px-2 py-0.5 text-xs rounded bg-bg-elevated text-text-secondary disabled:opacity-30 hover:bg-border-default transition-colors"
              >
                上一页
              </button>
              <span className="px-2 py-0.5 text-xs text-text-muted font-mono-num">
                {Math.floor(offset / limit) + 1} / {Math.ceil(total / limit)}
              </span>
              <button
                onClick={() => setOffset(offset + limit)}
                disabled={!data || offset + limit >= total}
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
