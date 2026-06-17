import { createPortal } from 'react-dom'
import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent, type WheelEvent } from 'react'
import { fetchHotVideos } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { useApi } from '../hooks/useApi'
import type { HotVideoItem, HotVideosSort } from '../types'
import { formatNumber, formatPercent } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'

const SORT_OPTIONS: Array<{ value: HotVideosSort; label: string }> = [
  { value: 'heat', label: '热度' },
  { value: 'views', label: '播放' },
  { value: 'likes', label: '点赞' },
  { value: 'coins', label: '投币' },
]

const FETCH_LIMIT = 50
const WINDOW_SIZE = 6
const MAX_VIDEO_MS = 20000

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}

export default function HotVideoTable() {
  const [windowStart, setWindowStart] = useState(0)
  const [hoveredVideo, setHoveredVideo] = useState<HotVideoItem | null>(null)
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 })

  const {
    selectedBvid,
    setSelectedBvid,
    selectedCategory,
    selectedWord,
    sort,
    setSort,
    carousel,
    startCarousel,
    stopCarousel,
    advanceCarousel,
    carouselAdvanceRef,
  } = useDashboard()

  const { data, loading } = useApi(
    () => fetchHotVideos(sort, FETCH_LIMIT, 0, selectedCategory ?? undefined),
    [sort, selectedCategory]
  )

  const videoStartRef = useRef(0)
  const prevSelectedBvid = useRef<string | null>(null)

  const videos = useMemo(() => {
    let nextVideos = data?.videos ?? []
    if (selectedWord) {
      nextVideos = nextVideos.filter((video) => video.title.includes(selectedWord))
    }
    return nextVideos
  }, [data, selectedWord])

  const maxWindowStart = Math.max(0, videos.length - WINDOW_SIZE)
  const safeWindowStart = clamp(windowStart, 0, maxWindowStart)
  const selectedIndex = selectedBvid ? videos.findIndex((video) => video.bvid === selectedBvid) : -1
  const focusWindowStart =
    selectedIndex >= 0 ? clamp(selectedIndex - Math.floor(WINDOW_SIZE / 2), 0, maxWindowStart) : safeWindowStart
  const effectiveWindowStart =
    selectedIndex >= 0 &&
    (carousel.on || selectedIndex < safeWindowStart || selectedIndex >= safeWindowStart + WINDOW_SIZE)
      ? focusWindowStart
      : safeWindowStart
  const visibleVideos = videos.slice(effectiveWindowStart, effectiveWindowStart + WINDOW_SIZE)
  const wordCloudDone = carousel.wordCloud.commentShown && carousel.wordCloud.danmakuShown

  const metrics = {
    heat: { label: '热度', getValue: (video: HotVideoItem) => video.heat_score.toFixed(3) },
    views: { label: '播放', getValue: (video: HotVideoItem) => formatNumber(video.view_count) },
    likes: { label: '点赞', getValue: (video: HotVideoItem) => formatNumber(video.like_count) },
    coins: { label: '投币', getValue: (video: HotVideoItem) => formatNumber(video.coin_count) },
  } as const

  const secondaryCols: Array<keyof typeof metrics> =
    sort === 'views' ? ['heat', 'likes'] : sort === 'likes' ? ['views', 'heat'] : ['views', 'likes']

  const centerWindowOnIndex = useCallback(
    (index: number) => {
      setWindowStart(clamp(index - Math.floor(WINDOW_SIZE / 2), 0, maxWindowStart))
    },
    [maxWindowStart]
  )

  const focusVideoByIndex = useCallback(
    (index: number) => {
      const nextVideo = videos[index]
      if (!nextVideo) return

      prevSelectedBvid.current = nextVideo.bvid
      videoStartRef.current = Date.now()
      centerWindowOnIndex(index)
      setSelectedBvid(nextVideo.bvid)
    },
    [videos, centerWindowOnIndex, setSelectedBvid]
  )

  const doAdvance = useCallback(() => {
    if (videos.length === 0) return

    const currentBvid = prevSelectedBvid.current ?? selectedBvid
    const currentIndex = currentBvid ? videos.findIndex((video) => video.bvid === currentBvid) : -1
    const nextIndex = currentIndex >= 0 ? (currentIndex + 1) % videos.length : 0

    focusVideoByIndex(nextIndex)
  }, [focusVideoByIndex, selectedBvid, videos])

  useEffect(() => {
    carouselAdvanceRef.current = doAdvance
  }, [carouselAdvanceRef, doAdvance])

  useEffect(() => {
    if (!carousel.on) return

    if (wordCloudDone && carousel.commentsEnded) {
      advanceCarousel()
      return
    }

    const remaining = MAX_VIDEO_MS - (Date.now() - videoStartRef.current)
    if (remaining <= 0) {
      advanceCarousel()
      return
    }

    const timeout = window.setTimeout(() => advanceCarousel(), remaining)
    return () => window.clearTimeout(timeout)
  }, [advanceCarousel, carousel.commentsEnded, carousel.on, wordCloudDone])

  useEffect(() => () => stopCarousel(), [stopCarousel])

  const handleToggleCarousel = useCallback(() => {
    if (carousel.on) {
      stopCarousel()
      return
    }

    focusVideoByIndex(selectedIndex >= 0 ? selectedIndex : 0)
    startCarousel()
  }, [carousel.on, focusVideoByIndex, selectedIndex, startCarousel, stopCarousel])

  const slideWindow = useCallback(
    (delta: number) => {
      if (videos.length <= WINDOW_SIZE) return
      setWindowStart((prev) => clamp(prev + delta, 0, maxWindowStart))
    },
    [maxWindowStart, videos.length]
  )

  const handleWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      event.preventDefault()
      event.stopPropagation()

      if (Math.abs(event.deltaY) < 8) return
      slideWindow(event.deltaY > 0 ? 1 : -1)
    },
    [slideWindow]
  )

  const handleMouseMove = useCallback((video: HotVideoItem | null, event: MouseEvent<HTMLTableRowElement>) => {
    setHoveredVideo(video)
    setMousePos({ x: event.clientX, y: event.clientY })
  }, [])

  const extra = (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={handleToggleCarousel}
        className={`inline-flex min-h-8 items-center gap-1 rounded-md px-2.5 py-1 text-xs transition-colors focus:outline-none focus:ring-2 focus:ring-bili-blue/60 ${
          carousel.on
            ? 'bg-bili-pink text-white'
            : 'border border-border-default text-text-muted hover:border-bili-blue hover:text-text-primary'
        }`}
      >
        {carousel.on ? (
          <>
            <span className="h-1.5 w-1.5 rounded-full bg-white animate-pulse" />
            轮播中
          </>
        ) : (
          '开启轮播'
        )}
      </button>

      <div className="flex items-center gap-1">
        {SORT_OPTIONS.map((option) => (
          <button
            key={option.value}
            type="button"
            onClick={() => {
              if (sort === option.value) return
              setSort(option.value)
              setWindowStart(0)
            }}
            className={`min-h-8 rounded-md px-2.5 py-1 text-xs transition-colors focus:outline-none focus:ring-2 focus:ring-bili-blue/60 ${
              sort === option.value ? 'bg-bili-blue text-white' : 'text-text-muted hover:text-text-primary'
            }`}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  )

  return (
    <Card title="热门视频" extra={extra} className="h-full">
      {loading ? (
        <Loading rows={6} />
      ) : (
        <div className="flex h-full min-h-0 flex-col gap-2">
          {(selectedCategory || selectedWord) && (
            <div className="flex flex-wrap items-center gap-2 text-[11px] text-text-muted">
              {selectedCategory && (
                <span className="rounded-full bg-bg-elevated/70 px-2.5 py-1 text-text-secondary">
                  当前分区：{selectedCategory}
                </span>
              )}
              {selectedWord && (
                <span className="rounded-full bg-bili-pink/12 px-2.5 py-1 text-bili-pink">关键词：{selectedWord}</span>
              )}
            </div>
          )}

          <div
            className="flex-1 min-h-0 overflow-hidden rounded-lg border border-border-default/35 bg-bg-primary/12"
            onWheel={handleWheel}
          >
            <table className="w-full table-fixed text-xs">
              <thead>
                <tr className="border-b border-border-default/80 text-text-muted">
                  <th className="w-9 py-2 text-left">#</th>
                  <th className="py-2 text-left">标题</th>
                  <th className="w-24 py-2 text-left">UP</th>
                  <th className="w-16 py-2 text-left">分区</th>
                  <th className="w-18 py-2 text-right">{metrics[secondaryCols[0]].label}</th>
                  <th className="w-18 py-2 text-right">{metrics[secondaryCols[1]].label}</th>
                  <th className="w-20 py-2 pr-3 text-right text-bili-pink">{metrics[sort].label}</th>
                </tr>
              </thead>

              <tbody>
                {visibleVideos.map((video, index) => {
                  const actualIndex = effectiveWindowStart + index
                  const isSelected = video.bvid === selectedBvid

                  return (
                    <tr
                      key={video.bvid}
                      onClick={() => {
                        if (carousel.on) return

                        prevSelectedBvid.current = video.bvid
                        centerWindowOnIndex(actualIndex)
                        setSelectedBvid(video.bvid === selectedBvid ? null : video.bvid)
                      }}
                      onMouseMove={(event) => {
                        if (!carousel.on) handleMouseMove(video, event)
                      }}
                      onMouseLeave={() => setHoveredVideo(null)}
                      className={`border-b border-border-default/25 transition-colors duration-200 ${
                        carousel.on ? 'cursor-default' : 'cursor-pointer'
                      } ${isSelected ? 'bg-bili-blue/10' : 'hover:bg-bg-elevated/65'}`}
                    >
                      <td className="py-2 font-mono-num text-text-muted">{actualIndex + 1}</td>
                      <td className="py-2 pr-2 text-text-primary">
                        <div className="truncate">{video.title}</div>
                      </td>
                      <td className="py-2 text-text-secondary">
                        <div className="truncate">{video.up_name}</div>
                      </td>
                      <td className="py-2 text-text-muted">
                        <span className="rounded bg-bg-elevated px-1.5 py-0.5 text-[10px]">{video.category}</span>
                      </td>
                      <td className="py-2 text-right font-mono-num text-text-secondary">
                        {metrics[secondaryCols[0]].getValue(video)}
                      </td>
                      <td className="py-2 text-right font-mono-num text-text-secondary">
                        {metrics[secondaryCols[1]].getValue(video)}
                      </td>
                      <td className="py-2 pr-3 text-right font-mono-num font-semibold text-bili-pink">
                        {metrics[sort].getValue(video)}
                      </td>
                    </tr>
                  )
                })}

                {visibleVideos.length === 0 && (
                  <tr>
                    <td colSpan={7} className="py-12 text-center text-sm text-text-muted">
                      暂无视频数据
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {hoveredVideo &&
        !carousel.on &&
        createPortal(<HoverCard video={hoveredVideo} mousePos={mousePos} />, document.body)}
    </Card>
  )
}

function HoverCard({ video, mousePos }: { video: HotVideoItem; mousePos: { x: number; y: number } }) {
  const rates =
    video.view_count > 0
      ? {
          like: video.like_count / video.view_count,
          coin: video.coin_count / video.view_count,
          favorite: video.favorite_count / video.view_count,
          reply: video.reply_count / video.view_count,
          share: video.share_count / video.view_count,
        }
      : null

  const width = 260
  const height = 220
  const gap = 14

  let left = mousePos.x + gap
  let top = mousePos.y + gap

  if (left + width > window.innerWidth - gap) left = mousePos.x - width - gap
  if (top + height > window.innerHeight - gap) top = mousePos.y - height - gap
  if (left < gap) left = gap
  if (top < gap) top = gap

  return (
    <div className="pointer-events-none" style={{ position: 'fixed', left, top, zIndex: 2147483647 }}>
      <div
        className="w-[260px] rounded-lg border border-bili-blue/30 bg-bg-card p-3 animate-fade-in"
        style={{ boxShadow: '0 8px 32px rgba(0,0,0,0.6), 0 0 0 1px rgba(0,161,214,0.2)' }}
      >
        <div className="mb-2 line-clamp-1 text-sm font-medium text-text-primary">{video.title}</div>
        <div className="mb-3 flex items-center gap-2">
          <span className="text-xs text-bili-blue">{video.up_name}</span>
          <span className="rounded bg-bg-elevated px-1.5 py-0.5 text-[10px] text-text-muted">{video.category}</span>
        </div>

        <div className="mb-3 grid grid-cols-3 gap-2">
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
            <div className="mb-1 text-[10px] text-text-muted">互动率</div>
            <div className="grid grid-cols-5 gap-1">
              {[
                { label: '赞', value: rates.like, color: '#FB7299' },
                { label: '币', value: rates.coin, color: '#39D353' },
                { label: '藏', value: rates.favorite, color: '#F0883E' },
                { label: '评', value: rates.reply, color: '#A371F7' },
                { label: '转', value: rates.share, color: '#79C0FF' },
              ].map((rate) => (
                <div key={rate.label} className="text-center">
                  <div className="text-[9px] text-text-muted">{rate.label}</div>
                  <div className="font-mono-num text-[10px]" style={{ color: rate.color }}>
                    {formatPercent(rate.value)}
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
