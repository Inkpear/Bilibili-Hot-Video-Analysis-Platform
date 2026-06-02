import { useState, useMemo, useRef, useEffect, useCallback } from 'react'
import { useApi } from '../hooks/useApi'
import { fetchHotVideos } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { formatNumber } from '../utils/format'
import Card from './common/Card'
import Loading from './common/Loading'
import type { HotVideosSort, HotVideoItem } from '../types'

const SORT_OPTIONS: { value: HotVideosSort; label: string }[] = [
  { value: 'heat', label: '热度' },
  { value: 'views', label: '播放' },
  { value: 'likes', label: '点赞' },
  { value: 'coins', label: '投币' },
]

const PAGE_SIZE = 20

export default function HotVideoTable() {
  const [sort, setSort] = useState<HotVideosSort>('heat')
  const [allVideos, setAllVideos] = useState<HotVideoItem[]>([])
  const [offset, setOffset] = useState(0)
  const [hasMore, setHasMore] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)

  const { data, loading } = useApi(() => fetchHotVideos(sort, PAGE_SIZE, 0), [sort])
  const { selectedBvid, setSelectedBvid, selectedCategory, selectedWord } = useDashboard()

  const scrollRef = useRef<HTMLDivElement>(null)
  const sentinelRef = useRef<HTMLDivElement>(null)
  const carouselRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const [carouselOn, setCarouselOn] = useState(false)
  const [carouselIdx, setCarouselIdx] = useState(0)

  useEffect(() => {
    if (data) {
      setAllVideos(data.videos)
      setOffset(PAGE_SIZE)
      setHasMore(data.videos.length < data.total)
    }
  }, [data])

  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return
    setLoadingMore(true)
    try {
      const res = await fetchHotVideos(sort, PAGE_SIZE, offset)
      setAllVideos((prev) => {
        const existing = new Set(prev.map((v) => v.bvid))
        const newItems = res.videos.filter((v) => !existing.has(v.bvid))
        return [...prev, ...newItems]
      })
      setOffset((prev) => prev + PAGE_SIZE)
      setHasMore(allVideos.length + res.videos.length < res.total)
    } finally {
      setLoadingMore(false)
    }
  }, [sort, offset, loadingMore, hasMore, allVideos.length])

  useEffect(() => {
    const sentinel = sentinelRef.current
    if (!sentinel) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) loadMore()
      },
      { root: scrollRef.current, threshold: 0.1 }
    )
    observer.observe(sentinel)
    return () => observer.disconnect()
  }, [loadMore])

  const videos = useMemo(() => {
    let list = allVideos
    if (selectedCategory) list = list.filter((v) => v.category === selectedCategory)
    if (selectedWord) list = list.filter((v) => v.title.includes(selectedWord))
    return list
  }, [allVideos, selectedCategory, selectedWord])

  const sortLabel = SORT_OPTIONS.find((s) => s.value === sort)?.label ?? '热度'

  const handleCarousel = useCallback(() => {
    if (carouselOn) {
      if (carouselRef.current) clearInterval(carouselRef.current)
      carouselRef.current = null
      setCarouselOn(false)
      return
    }
    setCarouselOn(true)
    setCarouselIdx(0)
    if (videos.length > 0) {
      setSelectedBvid(videos[0].bvid)
    }
    carouselRef.current = setInterval(() => {
      setCarouselIdx((prev) => {
        const next = (prev + 1) % videos.length
        setSelectedBvid(videos[next]?.bvid ?? null)
        return next
      })
    }, 3000)
  }, [carouselOn, videos, setSelectedBvid])

  useEffect(() => {
    return () => {
      if (carouselRef.current) clearInterval(carouselRef.current)
    }
  }, [])

  useEffect(() => {
    if (carouselOn && carouselRef.current) {
      clearInterval(carouselRef.current)
      setCarouselIdx(0)
      if (videos.length > 0) {
        setSelectedBvid(videos[0].bvid)
      }
      carouselRef.current = setInterval(() => {
        setCarouselIdx((prev) => {
          const next = (prev + 1) % videos.length
          setSelectedBvid(videos[next]?.bvid ?? null)
          return next
        })
      }, 3000)
    }
  }, [sort, selectedCategory, selectedWord]) // eslint-disable-line react-hooks/exhaustive-deps

  const extra = (
    <div className="flex items-center gap-2">
      <button
        onClick={handleCarousel}
        className={`flex items-center gap-1 px-2 py-0.5 text-xs rounded transition-all ${
          carouselOn
            ? 'bg-bili-pink text-white animate-pulse-glow'
            : 'text-text-muted hover:text-text-secondary border border-border-default hover:border-bili-blue'
        }`}
      >
        {carouselOn ? (
          <>
            <span className="w-1.5 h-1.5 rounded-full bg-white animate-pulse" />
            停止
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
            onClick={() => { setSort(s.value); setAllVideos([]); }}
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
            ref={scrollRef}
            className="flex-1 overflow-auto min-h-0 scrollbar-hide"
          >
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
                {videos.map((v, i) => {
                  const isSelected = v.bvid === selectedBvid
                  const isCarouselActive = carouselOn && i === carouselIdx
                  return (
                    <tr
                      key={v.bvid}
                      onClick={() => {
                        if (carouselOn) return
                        setSelectedBvid(v.bvid === selectedBvid ? null : v.bvid)
                      }}
                      className={`
                        border-b border-border-default/30 transition-all duration-300
                        ${carouselOn ? 'cursor-default' : 'cursor-pointer'}
                        ${isSelected
                          ? 'bg-bili-blue/10 border-l-2 border-l-bili-blue'
                          : 'hover:bg-bg-elevated'}
                        ${isCarouselActive ? 'animate-highlight-row' : ''}
                      `}
                    >
                      <td className="py-1.5 font-mono-num text-text-muted">{i + 1}</td>
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
                  )
                })}
              </tbody>
            </table>

            <div ref={sentinelRef} className="h-8 flex items-center justify-center">
              {loadingMore && (
                <div className="w-4 h-4 border-2 border-bili-blue border-t-transparent rounded-full animate-spin" />
              )}
              {!hasMore && videos.length > 0 && (
                <span className="text-[10px] text-text-muted">— 已加载全部 —</span>
              )}
            </div>
          </div>
        </div>
      )}
    </Card>
  )
}
