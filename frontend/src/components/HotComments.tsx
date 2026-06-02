import { useRef, useEffect, useCallback } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchHotComments } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'

export default function HotComments() {
  const { selectedCategory, selectedBvid, carouselOn, setReadyToSwitch } = useDashboard()
  const { data, loading } = useApi(
    () => fetchHotComments(selectedCategory ?? undefined, selectedBvid ?? undefined, 20),
    [selectedCategory, selectedBvid]
  )

  const scrollRef = useRef<HTMLDivElement>(null)
  const scrollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const hasReachedBottom = useRef(false)

  const handleScroll = useCallback(() => {
    const el = scrollRef.current
    if (!el || !carouselOn) return
    const atBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 10
    if (atBottom && !hasReachedBottom.current) {
      hasReachedBottom.current = true
      setReadyToSwitch(true)
    }
  }, [carouselOn, setReadyToSwitch])

  useEffect(() => {
    hasReachedBottom.current = false
  }, [selectedBvid])

  useEffect(() => {
    if (carouselOn && data && data.comments.length > 0) {
      let scrollTop = 0
      const el = scrollRef.current
      if (!el) return

      scrollTimerRef.current = setInterval(() => {
        if (!el) return
        scrollTop += 1
        el.scrollTop = scrollTop
        const atBottom = scrollTop >= el.scrollHeight - el.clientHeight - 10
        if (atBottom && !hasReachedBottom.current) {
          hasReachedBottom.current = true
          setReadyToSwitch(true)
        }
        if (atBottom) {
          scrollTop = 0
        }
      }, 50)

      return () => {
        if (scrollTimerRef.current) clearInterval(scrollTimerRef.current)
      }
    }
    if (!carouselOn) {
      if (scrollTimerRef.current) clearInterval(scrollTimerRef.current)
      scrollTimerRef.current = null
    }
  }, [carouselOn, data, setReadyToSwitch])

  return (
    <Card title="热门评论" className="h-full">
      {loading ? (
        <Loading rows={6} />
      ) : (
        <div
          ref={scrollRef}
          onScroll={handleScroll}
          className="flex flex-col gap-2 overflow-auto min-h-0 h-full scrollbar-hide"
        >
          {data?.comments.map((c, i) => (
            <div
              key={`${c.bvid}-${i}`}
              className="flex gap-3 py-2 border-b border-border-default/50 last:border-0"
            >
              <div className="flex-shrink-0 w-8 h-8 rounded-full bg-bg-elevated flex items-center justify-center text-xs font-mono-num text-bili-pink font-bold">
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
          ))}
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
