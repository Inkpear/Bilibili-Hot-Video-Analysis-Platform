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

const MAX_VIDEO_MS = 20000

export default function HotVideoTable() {
  const [sort, setSort] = useState<HotVideosSort>('heat')

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

  const sortLabel = SORT_OPTIONS.find((s) => s.value === sort)?.label ?? '热度'
  const wcDone = carousel.on && carousel.wordCloud.commentShown && carousel.wordCloud.danmakuShown

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
    <Card title="热门视频" extra={extra} className="h-full">
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
                      className={`
                        border-b border-border-default/30 transition-all duration-300
                        ${carousel.on ? 'cursor-default' : 'cursor-pointer'}
                        ${isSelected
                          ? 'bg-bili-blue/10 border-l-2 border-l-bili-blue animate-pulse-border'
                          : 'hover:bg-bg-elevated'}
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

            <div className="h-4 flex items-center justify-center">
              <span className="text-[10px] text-text-muted">共 {videos.length} 条</span>
            </div>
          </div>
        </div>
      )}
    </Card>
  )
}
