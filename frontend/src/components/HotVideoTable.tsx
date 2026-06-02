import { useState, useMemo, useRef, useEffect, useCallback } from 'react'
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

const MAX_VIDEO_MS = 20000

export default function HotVideoTable() {
  const [sort, setSort] = useState<HotVideosSort>('heat')
  const [hoveredVideo, setHoveredVideo] = useState<HotVideoItem | null>(null)
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number }>({ x: 0, y: 0 })

  const { data, loading } = useApi(() => fetchHotVideos(sort, 100, 0), [sort])
  const {
    selectedBvid, setSelectedBvid, selectedCategory, selectedWord,
    carousel, startCarousel, stopCarousel, advanceCarousel, carouselAdvanceRef,
  } = useDashboard()

  const scrollRef = useRef<HTMLDivElement>(null)
  const videosRef = useRef<HotVideoItem[]>([])
  const videoStartRef = useRef<number>(0)

  const videos = useMemo(() => {
    if (!data) return []
    let list = data.videos
    if (selectedCategory) list = list.filter((v) => v.category === selectedCategory)
    if (selectedWord) list = list.filter((v) => v.title.includes(selectedWord))
    return list
  }, [data, selectedCategory, selectedWord])

  useEffect(() => {
    videosRef.current = videos
  }, [videos])

  const doAdvance = useCallback(() => {
    const vids = videosRef.current
    if (vids.length === 0) return
    const currentBvid = selectedBvid
    const currentIdx = vids.findIndex((v) => v.bvid === currentBvid)
    const next = (currentIdx + 1) % vids.length
    setSelectedBvid(vids[next]?.bvid ?? null)
    videoStartRef.current = Date.now()
  }, [setSelectedBvid, selectedBvid])

  useEffect(() => {
    carouselAdvanceRef.current = doAdvance
  }, [doAdvance, carouselAdvanceRef])

  useEffect(() => {
    if (!carousel.on) return

    const { commentShown, danmakuShown } = carousel.wordCloud
    const wcDone = commentShown && danmakuShown
    const commentsDone = carousel.commentsEnded

    if (wcDone && commentsDone) {
      advanceCarousel()
      return
    }

    const elapsed = Date.now() - videoStartRef.current
    const remaining = MAX_VIDEO_MS - elapsed
    if (remaining <= 0) {
      advanceCarousel()
      return
    }

    const timeout = setTimeout(() => {
      advanceCarousel()
    }, remaining)

    return () => clearTimeout(timeout)
  }, [carousel.on, carousel.wordCloud.commentShown, carousel.wordCloud.danmakuShown, carousel.commentsEnded, advanceCarousel])

  const handleToggleCarousel = useCallback(() => {
    if (carousel.on) {
      stopCarousel()
      return
    }
    videoStartRef.current = Date.now()
    if (videos.length > 0) {
      setSelectedBvid(videos[0].bvid)
    }
    startCarousel()
  }, [carousel.on, videos, setSelectedBvid, startCarousel, stopCarousel])

  useEffect(() => {
    return () => stopCarousel()
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  const handleRowHover = useCallback((video: HotVideoItem | null, e?: React.MouseEvent) => {
    setHoveredVideo(video)
    if (e) {
      setHoverPos({ x: e.clientX, y: e.clientY })
    }
  }, [])

  const sortLabel = SORT_OPTIONS.find((s) => s.value === sort)?.label ?? '热度'
  const wcDone = carousel.on && carousel.wordCloud.commentShown && carousel.wordCloud.danmakuShown

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
            onClick={() => setSort(s.value)}
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
    <Card title="热门视频" extra={extra} className="h-full relative">
      {loading ? (
        <Loading rows={8} />
      ) : (
        <div className="flex flex-col h-full min-h-0">
          {selectedWord && (
            <div className="px-1 pb-2 text-xs text-text-muted shrink-0">
              标题包含 <span className="text-bili-pink font-medium">「{selectedWord}」</span> 的视频
            </div>
          )}

          <div ref={scrollRef} className="flex-1 overflow-auto min-h-0 scrollbar-hide">
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
                      ref={isSelected ? (el) => {
                        if (el) setTimeout(() => el.scrollIntoView({ behavior: 'smooth', block: 'nearest' }), 100)
                      } : undefined}
                      onClick={() => {
                        if (carousel.on) return
                        setSelectedBvid(v.bvid === selectedBvid ? null : v.bvid)
                      }}
                      onMouseEnter={(e) => handleRowHover(v, e)}
                      onMouseLeave={() => handleRowHover(null)}
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

            <div className="h-4 flex items-center justify-center">
              <span className="text-[10px] text-text-muted">共 {videos.length} 条</span>
            </div>
          </div>
        </div>
      )}

      {hoveredVideo && !carousel.on && (
        <HoverCard video={hoveredVideo} pos={hoverPos} />
      )}
    </Card>
  )
}

function HoverCard({ video, pos }: { video: HotVideoItem; pos: { x: number; y: number } }) {
  const cardRef = useRef<HTMLDivElement>(null)
  const [adjusted, setAdjusted] = useState(false)

  useEffect(() => {
    if (cardRef.current) {
      const rect = cardRef.current.getBoundingClientRect()
      if (rect.right > window.innerWidth) {
        setAdjusted(true)
      }
    }
  }, [pos])

  const rates = video.view_count > 0 ? {
    like: video.like_count / video.view_count,
    coin: video.coin_count / video.view_count,
    fav: video.favorite_count / video.view_count,
    reply: video.reply_count / video.view_count,
    share: video.share_count / video.view_count,
  } : null

  return (
    <div
      ref={cardRef}
      className="fixed z-50 pointer-events-none animate-fade-in"
      style={{
        left: adjusted ? pos.x - 280 : pos.x + 16,
        top: pos.y - 40,
      }}
    >
      <div className="bg-bg-card border border-border-bili rounded-lg shadow-2xl p-3 w-[260px]">
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
