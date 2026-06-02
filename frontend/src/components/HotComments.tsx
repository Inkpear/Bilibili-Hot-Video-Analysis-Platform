import { useRef, useEffect, useState } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchHotComments } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'

export default function HotComments() {
  const { selectedCategory, selectedBvid, carousel, markCommentsEnded } = useDashboard()
  const { data, loading } = useApi(
    () => fetchHotComments(selectedCategory ?? undefined, selectedBvid ?? undefined, 20),
    [selectedCategory, selectedBvid]
  )

  const scrollRef = useRef<HTMLDivElement>(null)
  const scrollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const [activeIdx, setActiveIdx] = useState(0)
  const hasEndedRef = useRef(false)

  useEffect(() => {
    hasEndedRef.current = false
    setActiveIdx(0)
  }, [selectedBvid])

  useEffect(() => {
    if (!carousel.on || !data || data.comments.length === 0) {
      if (scrollTimerRef.current) clearInterval(scrollTimerRef.current)
      scrollTimerRef.current = null
      return
    }

    const comments = data.comments
    const totalItems = comments.length
    let currentIdx = 0

    scrollTimerRef.current = setInterval(() => {
      const el = scrollRef.current
      if (!el) return

      currentIdx = (currentIdx + 1) % totalItems
      setActiveIdx(currentIdx)

      const targetRow = el.querySelector(`[data-comment-idx="${currentIdx}"]`)
      if (targetRow) {
        targetRow.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
      }

      if (currentIdx === totalItems - 1 && !hasEndedRef.current) {
        hasEndedRef.current = true
        markCommentsEnded()
      }
    }, 2000)

    return () => {
      if (scrollTimerRef.current) clearInterval(scrollTimerRef.current)
    }
  }, [carousel.on, data, markCommentsEnded])

  const isCarouselActive = carousel.on && !!selectedBvid

  return (
    <Card title="热门评论" className="h-full">
      {loading ? (
        <Loading rows={6} />
      ) : (
        <div ref={scrollRef} className="flex flex-col gap-1 overflow-auto min-h-0 h-full scrollbar-hide">
          {data?.comments.map((c, i) => {
            const isActive = isCarouselActive && i === activeIdx
            return (
              <div
                key={`${c.bvid}-${i}`}
                data-comment-idx={i}
                className={`
                  flex gap-3 py-2 px-2 rounded transition-all duration-300
                  ${isActive
                    ? 'bg-bili-blue/10 border border-bili-blue/30'
                    : 'border border-transparent'}
                `}
              >
                <div className={`
                  flex-shrink-0 w-7 h-7 rounded-full flex items-center justify-center
                  text-xs font-mono-num font-bold transition-colors
                  ${isActive ? 'bg-bili-blue text-white' : 'bg-bg-elevated text-bili-pink'}
                `}>
                  {i + 1}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-xs text-bili-blue font-medium truncate max-w-[120px]">{c.user_name}</span>
                    <span className="text-xs text-text-muted truncate max-w-[150px]">{c.title}</span>
                  </div>
                  <p className="text-sm text-text-primary leading-relaxed line-clamp-2">{c.content}</p>
                  <div className="flex items-center gap-3 mt-1">
                    <span className="text-xs text-bili-pink">
                      ♡ {formatNumber(c.like_count)}
                    </span>
                    <span className="text-xs text-text-muted">
                      回复 {c.reply_count}
                    </span>
                    {c.comment_time && (
                      <span className="text-xs text-text-muted">{c.comment_time}</span>
                    )}
                  </div>
                </div>
              </div>
            )
          })}
          {data?.comments.length === 0 && (
            <div className="flex items-center justify-center h-full text-text-muted text-sm">
              暂无评论数据
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
