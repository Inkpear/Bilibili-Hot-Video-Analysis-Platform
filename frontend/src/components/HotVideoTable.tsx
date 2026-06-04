import { useState, useMemo, useRef, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useApi } from '../hooks/useApi'
import { fetchHotVideos } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatNumber, formatPercent } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'
import type { HotVideosSort, HotVideoItem } from '../types'

const SORT_OPTIONS: { value: HotVideosSort; label: string }[] = [
  { value: 'heat', label: '热度' },
  { value: 'views', label: '播放' },
  { value: 'likes', label: '点赞' },
  { value: 'coins', label: '投币' },
]

const BATCH_SIZE = 50
const MAX_VIDEO_MS = 20000
const PRELOAD_THRESHOLD = 5

export default function HotVideoTable() {
  const [sort, setSort] = useState<HotVideosSort>('heat')
  const [allVideos, setAllVideos] = useState<HotVideoItem[]>([])
  const [loadingMore, setLoadingMore] = useState(false)
  const [hoveredVideo, setHoveredVideo] = useState<HotVideoItem | null>(null)
  const [mousePos, setMousePos] = useState<{ x: number; y: number }>({ x: 0, y: 0 })

  const { data, loading } = useApi(() => fetchHotVideos(sort, BATCH_SIZE, 0, selectedCategory), [sort, selectedCategory])
  const {
    selectedBvid, setSelectedBvid, selectedCategory, selectedWord,
    carousel, startCarousel, stopCarousel, advanceCarousel, carouselAdvanceRef,
  } = useDashboard()

  const containerRef = useRef<HTMLDivElement>(null)
  const rowRefs = useRef<Map<string, HTMLTableRowElement>>(new Map())
  const isLoadingRef = useRef(false)
  const offsetRef = useRef(0)
  const hasMoreRef = useRef(true)
  const videosRef = useRef<HotVideoItem[]>([])
  const videoStartRef = useRef<number>(0)
  const prevSelectedBvid = useRef<string | null>(null)

  // ---- filtered videos ----
  const videos = useMemo(() => {
    let list = allVideos
    if (selectedWord) list = list.filter((v) => v.title.includes(selectedWord))
    return list
  }, [allVideos, selectedWord])

  useEffect(() => { videosRef.current = videos }, [videos])

  // ---- init on data change ----
  useEffect(() => {
    if (data) {
      setAllVideos(data.videos)
      offsetRef.current = BATCH_SIZE
      hasMoreRef.current = BATCH_SIZE < data.total
    }
  }, [data])

  // ---- 1. unified loader with lock ----
  const loadMore = useCallback(async () => {
    if (isLoadingRef.current || !hasMoreRef.current) return
    isLoadingRef.current = true
    setLoadingMore(true)
    try {
      const curOffset = offsetRef.current
      const res = await fetchHotVideos(sort, BATCH_SIZE, curOffset, selectedCategory)
      setAllVideos((prev) => {
        const existing = new Set(prev.map((v) => v.bvid))
        const newItems = res.videos.filter((v) => !existing.has(v.bvid))
        return [...prev, ...newItems]
      })
      offsetRef.current = curOffset + BATCH_SIZE
      hasMoreRef.current = curOffset + BATCH_SIZE < res.total
    } finally {
      isLoadingRef.current = false
      setLoadingMore(false)
    }
  }, [sort, selectedCategory])

  // ---- 2. manual scroll detection ----
  const handleScroll = useCallback(() => {
    const el = containerRef.current
    if (!el) return
    const { scrollTop, clientHeight, scrollHeight } = el
    if (scrollTop + clientHeight >= scrollHeight - 100) {
      loadMore()
    }
  }, [loadMore])

  // ---- 3. focus centering algorithm ----
  const scrollToFocus = useCallback((bvid: string) => {
    const container = containerRef.current
    const row = rowRefs.current.get(bvid)
    if (!container || !row) return

    const targetTop = row.offsetTop - container.clientHeight / 2 + row.clientHeight / 2
    container.scrollTo({
      top: Math.max(0, targetTop),
      behavior: 'smooth',
    })
  }, [])

  // ---- 4. scroll to selected on bvid change (non-carousel) ----
  useEffect(() => {
    if (carousel.on) return
    if (!selectedBvid || selectedBvid === prevSelectedBvid.current) return
    prevSelectedBvid.current = selectedBvid
    scrollToFocus(selectedBvid)
  }, [selectedBvid, carousel.on, scrollToFocus])

  // ---- 5. carousel advance engine ----
  const doAdvance = useCallback(() => {
    const vids = videosRef.current
    if (vids.length === 0) return

    const curBvid = prevSelectedBvid.current
    const curIdx = vids.findIndex((v) => v.bvid === curBvid)
    const nextIdx = (curIdx + 1) % vids.length

    // pre-load when approaching end
    if (nextIdx >= vids.length - PRELOAD_THRESHOLD) {
      loadMore()
    }

    // if wrapped around and no more data, stay at last
    if (nextIdx >= vids.length && !hasMoreRef.current) return

    const nextBvid = vids[nextIdx >= vids.length ? 0 : nextIdx]?.bvid ?? null
    prevSelectedBvid.current = nextBvid
    setSelectedBvid(nextBvid)
    videoStartRef.current = Date.now()

    // scroll to the new focus
    if (nextBvid) {
      requestAnimationFrame(() => scrollToFocus(nextBvid))
    }
  }, [setSelectedBvid, loadMore, scrollToFocus])

  useEffect(() => {
    carouselAdvanceRef.current = doAdvance
  }, [doAdvance, carouselAdvanceRef])

  // ---- 6. carousel timer (driven by word cloud + comments signals) ----
  useEffect(() => {
    if (!carousel.on) return
    const { commentShown, danmakuShown } = carousel.wordCloud
    const wcDone = commentShown && danmakuShown
    const commentsDone = carousel.commentsEnded
    if (wcDone && commentsDone) { advanceCarousel(); return }
    const elapsed = Date.now() - videoStartRef.current
    const remaining = MAX_VIDEO_MS - elapsed
    if (remaining <= 0) { advanceCarousel(); return }
    const timeout = setTimeout(() => advanceCarousel(), remaining)
    return () => clearTimeout(timeout)
  }, [carousel.on, carousel.wordCloud.commentShown, carousel.wordCloud.danmakuShown, carousel.commentsEnded, advanceCarousel])

  // ---- 7. toggle carousel ----
  const handleToggleCarousel = useCallback(() => {
    if (carousel.on) {
      stopCarousel()
      return
    }
    videoStartRef.current = Date.now()
    if (videos.length > 0) {
      const firstBvid = videos[0].bvid
      prevSelectedBvid.current = firstBvid
      setSelectedBvid(firstBvid)
      requestAnimationFrame(() => scrollToFocus(firstBvid))
    }
    startCarousel()
  }, [carousel.on, videos, setSelectedBvid, startCarousel, stopCarousel, scrollToFocus])

  useEffect(() => () => stopCarousel(), []) // eslint-disable-line react-hooks/exhaustive-deps

  // ---- 8. hover ----
  const handleMouseMove = useCallback((video: HotVideoItem | null, e: React.MouseEvent) => {
    setHoveredVideo(video)
    setMousePos({ x: e.clientX, y: e.clientY })
  }, [])

  // ---- register row ref ----
  const setRowRef = useCallback((bvid: string, el: HTMLTableRowElement | null) => {
    if (el) {
      rowRefs.current.set(bvid, el)
    } else {
      rowRefs.current.delete(bvid)
    }
  }, [])

  const sortLabel = SORT_OPTIONS.find((s) => s.value === sort)?.label ?? '热度'
  const wcDone = carousel.on && carousel.wordCloud.commentShown && carousel.wordCloud.danmakuShown
  const hasMore = hasMoreRef.current

  const getSortValue = (v: HotVideoItem) => {
    switch (sort) {
      case 'views': return v.view_count
      case 'likes': return v.like_count
      case 'coins': return v.coin_count
      default: return v.heat_score
    }
  }

  const extra = (
    <div className="flex items-center gap-2">
      <button
        onClick={handleToggleCarousel}
        className={`flex items-center gap-1 px-2 py-0.5 text-xs rounded transition-all ${
          carousel.on
            ? 'bg-bili-pink text-white'
            : 'text-text-muted hover:text-text-secondary border border-border-default hover:border-bili-blue'
        }`}
      >
        {carousel.on ? (
          <>
            <span className="w-1.5 h-1.5 rounded-full bg-white animate-pulse" />
            停止
            {wcDone && <span className="text-[10px] ml-0.5 opacity-70">词云✓</span>}
            {carousel.commentsEnded && <span className="text-[10px] ml-0.5 opacity-70">评论✓</span>}
          </>
        ) : (
          <>
            <span>▶</span>
            轮播
          </>
        )}
      </button>
      <div className="flex gap-1">
        {SORT_OPTIONS.map((s) => (
          <button
            key={s.value}
            onClick={() => {
              setSort(s.value)
              setAllVideos([])
              offsetRef.current = 0
              hasMoreRef.current = true
            }}
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
    </div>
  )

  return (
    <Card title="热门视频" extra={extra} className="h-full">
      {loading && allVideos.length === 0 ? (
        <Loading rows={8} />
      ) : (
        <div className="flex flex-col h-full min-h-0">
          {selectedWord && (
            <div className="px-1 pb-2 text-xs text-text-muted shrink-0">
              标题包含 <span className="text-bili-pink font-medium">「{selectedWord}」</span> 的视频
            </div>
          )}

          <div
            ref={containerRef}
            onScroll={handleScroll}
            className="flex-1 overflow-auto min-h-0 scrollbar-hide"
          >
            <table className="w-full text-xs">
              <thead className="sticky top-0 bg-bg-card z-10">
                <tr className="text-text-muted border-b border-border-default">
                  <th className="py-1.5 text-left w-8">#</th>
                  <th className="py-1.5 text-left">标题</th>
                  <th className="py-1.5 text-left">UP主</th>
                  <th className="py-1.5 text-left">分区</th>
                  <th className="py-1.5 text-right">播放</th>
                  <th className="py-1.5 text-right">点赞</th>
                  <th className="py-1.5 text-right">{sortLabel}</th>
                </tr>
              </thead>
              <tbody>
                {videos.map((v, i) => {
                  const isSelected = v.bvid === selectedBvid
                  return (
                    <tr
                      key={v.bvid}
                      ref={(el) => setRowRef(v.bvid, el)}
                      onClick={() => {
                        if (carousel.on) return
                        setSelectedBvid(v.bvid === selectedBvid ? null : v.bvid)
                      }}
                      onMouseMove={(e) => {
                        if (!carousel.on) handleMouseMove(v, e)
                      }}
                      onMouseLeave={() => setHoveredVideo(null)}
                      className={`
                        border-b border-border-default/30 transition-all duration-300
                        ${carousel.on ? 'cursor-default' : 'cursor-pointer'}
                        ${isSelected
                          ? 'bg-bili-blue/10 border-l-2 border-l-bili-blue animate-pulse-border'
                          : 'hover:bg-bg-elevated'}
                      `}
                    >
                      <td className="py-1.5 font-mono-num text-text-muted">{i + 1}</td>
                      <td className="py-1.5 max-w-[180px] truncate text-text-primary">{v.title}</td>
                      <td className="py-1.5 text-text-muted max-w-[80px] truncate">{v.up_name}</td>
                      <td className="py-1.5 text-text-muted">
                        <span className="px-1.5 py-0.5 rounded bg-bg-elevated text-[10px]">{v.category}</span>
                      </td>
                      <td className="py-1.5 text-right font-mono-num text-text-secondary">{formatNumber(v.view_count)}</td>
                      <td className="py-1.5 text-right font-mono-num text-text-secondary">{formatNumber(v.like_count)}</td>
                      <td className="py-1.5 text-right font-mono-num text-bili-pink font-semibold">
                        {sort === 'heat' ? v.heat_score.toFixed(3) : formatNumber(getSortValue(v))}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>

            <div className="h-8 flex items-center justify-center">
              {loadingMore && (
                <div className="w-4 h-4 border-2 border-bili-blue border-t-transparent rounded-full animate-spin" />
              )}
              {!hasMore && videos.length > 0 && (
                <span className="text-[10px] text-text-muted">— 已加载全部 {videos.length} 条 —</span>
              )}
            </div>
          </div>
        </div>
      )}

      {hoveredVideo && !carousel.on && createPortal(
        <HoverCard video={hoveredVideo} mousePos={mousePos} />,
        document.body
      )}
    </Card>
  )
}

function HoverCard({ video, mousePos }: { video: HotVideoItem; mousePos: { x: number; y: number } }) {
  const rates = video.view_count > 0 ? {
    like: video.like_count / video.view_count,
    coin: video.coin_count / video.view_count,
    fav: video.favorite_count / video.view_count,
    reply: video.reply_count / video.view_count,
    share: video.share_count / video.view_count,
  } : null

  const W = 260
  const H = 220
  const GAP = 14

  let left = mousePos.x + GAP
  let top = mousePos.y + GAP

  if (left + W > window.innerWidth - GAP) left = mousePos.x - W - GAP
  if (top + H > window.innerHeight - GAP) top = mousePos.y - H - GAP
  if (left < GAP) left = GAP
  if (top < GAP) top = GAP

  return (
    <div
      className="pointer-events-none"
      style={{ position: 'fixed', left, top, zIndex: 2147483647 }}
    >
      <div
        className="bg-bg-card border border-bili-blue/30 rounded-lg p-3 w-[260px] animate-fade-in"
        style={{ boxShadow: '0 8px 32px rgba(0,0,0,0.6), 0 0 0 1px rgba(0,161,214,0.2)' }}
      >
        <div className="text-sm font-medium text-text-primary mb-2 line-clamp-1">{video.title}</div>
        <div className="flex items-center gap-2 mb-3">
          <span className="text-xs text-bili-blue">{video.up_name}</span>
          <span className="text-[10px] text-text-muted px-1.5 py-0.5 rounded bg-bg-elevated">{video.category}</span>
        </div>

        <div className="grid grid-cols-3 gap-2 mb-3">
          {[
            { label: '播放', value: video.view_count, color: '#00A1D6' },
            { label: '点赞', value: video.like_count, color: '#FB7299' },
            { label: '投币', value: video.coin_count, color: '#39D353' },
            { label: '收藏', value: video.favorite_count, color: '#F0883E' },
            { label: '评论', value: video.reply_count, color: '#A371F7' },
            { label: '分享', value: video.share_count, color: '#79C0FF' },
          ].map((item) => (
            <div key={item.label} className="text-center">
              <div className="text-[10px] text-text-muted">{item.label}</div>
              <div className="font-mono-num text-xs font-semibold" style={{ color: item.color }}>
                {formatNumber(item.value)}
              </div>
            </div>
          ))}
        </div>

        {rates && (
          <div className="border-t border-border-default pt-2">
            <div className="text-[10px] text-text-muted mb-1">互动率</div>
            <div className="grid grid-cols-5 gap-1">
              {[
                { label: '赞', value: rates.like, color: '#FB7299' },
                { label: '币', value: rates.coin, color: '#39D353' },
                { label: '藏', value: rates.fav, color: '#F0883E' },
                { label: '评', value: rates.reply, color: '#A371F7' },
                { label: '享', value: rates.share, color: '#79C0FF' },
              ].map((r) => (
                <div key={r.label} className="text-center">
                  <div className="text-[9px] text-text-muted">{r.label}</div>
                  <div className="font-mono-num text-[10px]" style={{ color: r.color }}>
                    {formatPercent(r.value)}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
