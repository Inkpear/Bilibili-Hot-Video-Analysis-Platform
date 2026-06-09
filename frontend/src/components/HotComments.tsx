import { useCallback, useEffect, useMemo, useRef, useState, type WheelEvent } from 'react'
import { fetchHotComments } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { useApi } from '../hooks/useApi'
import type { HotCommentItem } from '../types'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'

const WINDOW_SIZE = 4
const STEP_MS = 2000

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}

export default function HotComments() {
  const { selectedCategory, selectedBvid, carousel, markCommentsEnded, selectedSnapshotDate } = useDashboard()
  const [sort, setSort] = useState<string>('likes')
  const { data, loading } = useApi(
    () => fetchHotComments(selectedCategory ?? undefined, selectedBvid ?? undefined, 20, sort, selectedSnapshotDate),
    [selectedCategory, selectedBvid, sort, selectedSnapshotDate]
  )

  const comments = data?.comments ?? []
  const isCarouselActive = carousel.on && !!selectedBvid
  const scopeKey = `${selectedCategory ?? 'all'}-${selectedBvid ?? 'all'}-${isCarouselActive ? 'auto' : 'manual'}-${sort}`

  const scopeLabel = useMemo(() => {
    if (selectedBvid) return '当前视频评论'
    if (selectedCategory) return `${selectedCategory}分区评论`
    return '全样本热门评论'
  }, [selectedBvid, selectedCategory])

  const extra = (
    <div className="flex items-center gap-1">
      {[
        { value: 'likes', label: '点赞' },
        { value: 'replies', label: '回复' },
      ].map((opt) => (
        <button
          key={opt.value}
          type="button"
          onClick={() => setSort(opt.value)}
          className={`min-h-7 rounded-md px-2 py-0.5 text-xs transition-colors ${
            sort === opt.value ? 'bg-bili-blue text-white' : 'text-text-muted hover:text-text-primary'
          }`}
        >
          {opt.label}
        </button>
      ))}
    </div>
  )

  return (
    <Card title="热门评论" extra={extra} className="h-full">
      {loading ? (
        <Loading rows={4} />
      ) : comments.length > 0 ? (
        <CommentWindow
          key={scopeKey}
          comments={comments}
          autoPlay={isCarouselActive}
          onSequenceEnd={markCommentsEnded}
          scopeLabel={scopeLabel}
        />
      ) : (
        <div className="flex h-full items-center justify-center text-sm text-text-muted">暂无评论数据</div>
      )}
    </Card>
  )
}

function CommentWindow({
  comments,
  autoPlay,
  onSequenceEnd,
  scopeLabel,
}: {
  comments: HotCommentItem[]
  autoPlay: boolean
  onSequenceEnd: () => void
  scopeLabel: string
}) {
  const [windowStart, setWindowStart] = useState(0)
  const [activeIdx, setActiveIdx] = useState(0)
  const endedRef = useRef(false)

  const maxWindowStart = Math.max(0, comments.length - WINDOW_SIZE)
  const safeWindowStart = clamp(windowStart, 0, maxWindowStart)
  const safeActiveIdx = clamp(activeIdx, 0, Math.max(comments.length - 1, 0))
  const effectiveWindowStart = autoPlay ? clamp(safeActiveIdx - 1, 0, maxWindowStart) : safeWindowStart
  const visibleComments = comments.slice(effectiveWindowStart, effectiveWindowStart + WINDOW_SIZE)

  useEffect(() => {
    if (!autoPlay || comments.length === 0) return

    endedRef.current = false

    let currentIdx = 0
    const timer = window.setInterval(() => {
      currentIdx = (currentIdx + 1) % comments.length
      setActiveIdx(currentIdx)

      if (currentIdx === comments.length - 1 && !endedRef.current) {
        endedRef.current = true
        onSequenceEnd()
      }
    }, STEP_MS)

    return () => window.clearInterval(timer)
  }, [autoPlay, comments, onSequenceEnd])

  const slideWindow = useCallback(
    (delta: number) => {
      if (autoPlay || comments.length <= WINDOW_SIZE) return
      setWindowStart((prev) => clamp(prev + delta, 0, maxWindowStart))
    },
    [autoPlay, comments.length, maxWindowStart]
  )

  const handleWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      event.preventDefault()
      event.stopPropagation()

      if (autoPlay || Math.abs(event.deltaY) < 8) return
      slideWindow(event.deltaY > 0 ? 1 : -1)
    },
    [autoPlay, slideWindow]
  )

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="text-[11px] text-text-muted">{scopeLabel}</div>

      <div className="flex-1 min-h-0 space-y-1.5 overflow-hidden" onWheel={handleWheel}>
        {visibleComments.map((comment, index) => {
          const actualIndex = effectiveWindowStart + index
          const isActive = autoPlay && actualIndex === safeActiveIdx

          return (
            <div
              key={`${comment.bvid}-${actualIndex}`}
              className={`rounded-lg px-2 py-2 transition-colors duration-200 ${
                isActive
                  ? 'bg-bili-blue/10 ring-1 ring-bili-blue/30'
                  : 'bg-bg-elevated/40 hover:bg-bg-elevated/55'
              }`}
            >
              <div className="flex items-start gap-3">
                <div
                  className={`flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-full text-xs font-bold font-mono-num ${
                    isActive ? 'bg-bili-blue text-white' : 'bg-bg-elevated text-bili-pink'
                  }`}
                >
                  {actualIndex + 1}
                </div>

                <div className="min-w-0 flex-1">
                  <div className="mb-1 flex items-center gap-2">
                    <span className="max-w-[96px] truncate text-xs font-medium text-bili-blue">
                      {comment.user_name}
                    </span>
                    <span className="max-w-[180px] truncate text-xs text-text-muted">{comment.title}</span>
                  </div>

                  <p className="line-clamp-2 text-sm leading-relaxed text-text-primary">{comment.content}</p>

                  <div className="mt-1.5 flex items-center gap-3 text-xs text-text-muted">
                    <span className="text-bili-pink">点赞 {formatNumber(comment.like_count)}</span>
                    <span>回复 {comment.reply_count}</span>
                    {comment.comment_time && <span className="truncate">{comment.comment_time}</span>}
                  </div>
                </div>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
