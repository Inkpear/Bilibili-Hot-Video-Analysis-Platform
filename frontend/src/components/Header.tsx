import { useState, useRef, useEffect } from 'react'
import { createPortal } from 'react-dom'
import { useApi } from '../hooks/useApi'
import { fetchOverview, fetchSnapshotDates } from '../api'
import { useDashboard } from '../context/DashboardContext'
import { LayoutDashboard, BarChart3, Calendar, ChevronDown } from 'lucide-react'

interface HeaderProps {
  screen: 'overview' | 'analysis'
  onScreenChange: (screen: 'overview' | 'analysis') => void
}

const TABS: { key: 'overview' | 'analysis'; label: string; icon: typeof LayoutDashboard }[] = [
  { key: 'overview', label: '总览', icon: LayoutDashboard },
  { key: 'analysis', label: '分析', icon: BarChart3 },
]

export default function Header({ screen, onScreenChange }: HeaderProps) {
  const { data } = useApi(fetchOverview)
  const { data: datesData } = useApi(fetchSnapshotDates)
  const { selectedSnapshotDate, setSelectedSnapshotDate } = useDashboard()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const btnRef = useRef<HTMLButtonElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState({ top: 0, left: 0 })

  useEffect(() => {
    if (open && btnRef.current) {
      const r = btnRef.current.getBoundingClientRect()
      setPos({ top: r.bottom + 4, left: r.left })
    }
  }, [open])

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as Node
      if (ref.current && !ref.current.contains(target) && !menuRef.current?.contains(target)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  const dates = datesData?.dates ?? []

  return (
    <header className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
      <div className="flex items-center gap-4">
        <div className="relative">
          <div className="w-1.5 h-8 bg-gradient-to-b from-bili-blue to-bili-pink rounded-full" />
          <div className="absolute inset-0 w-1.5 h-8 bg-gradient-to-b from-bili-blue to-bili-pink rounded-full blur-md opacity-60" />
        </div>
        <div>
          <h1 className="font-display text-2xl tracking-wider text-text-primary leading-none">
            <span className="glow-blue">BILIBILI</span>
            <span className="text-bili-blue ml-2 glow-blue">数据终端</span>
          </h1>
          <p className="text-[10px] text-text-muted tracking-widest mt-0.5 font-mono-num">
            HOT VIDEO ANALYSIS PLATFORM
          </p>
        </div>

        <div className="ml-6 flex gap-1 rounded-lg bg-bg-card border border-border-default p-1">
          {TABS.map((tab) => {
            const active = screen === tab.key
            return (
              <button
                key={tab.key}
                onClick={() => onScreenChange(tab.key)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-200 ${
                  active
                    ? 'bg-bili-blue text-white shadow-sm'
                    : 'text-text-muted hover:text-text-secondary hover:bg-bg-elevated/50'
                }`}
              >
                <tab.icon size={14} />
                {tab.label}
              </button>
            )
          })}
        </div>

        {dates.length > 0 && (
          <div ref={ref} className="relative ml-4">
            <button
              ref={btnRef}
              onClick={() => setOpen(!open)}
              className={`flex items-center gap-1.5 rounded-lg bg-bg-card border px-3 py-1.5 text-xs font-medium transition-all duration-200 ${
                open
                  ? 'border-bili-blue/50 shadow-sm shadow-bili-blue/10'
                  : 'border-border-default hover:border-bili-blue/30 hover:shadow-sm hover:shadow-bili-blue/5'
              }`}
            >
              <Calendar size={13} className={selectedSnapshotDate ? 'text-bili-blue' : 'text-text-muted'} />
              <span className={selectedSnapshotDate ? 'text-bili-blue' : 'text-text-muted'}>
                {selectedSnapshotDate || '最新快照'}
              </span>
              <ChevronDown
                size={12}
                className={`transition-transform duration-200 ${open ? 'rotate-180 text-bili-blue' : 'text-text-muted'}`}
              />
            </button>
          </div>
        )}
      </div>

      {data && (
        <div className="flex flex-wrap items-center gap-4 xl:gap-6">
          <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-bg-card border border-border-default">
            <div className="w-1.5 h-1.5 rounded-full bg-positive animate-pulse" />
            <span className="text-[10px] text-text-muted">LIVE</span>
          </div>
          <div className="text-right">
            <div className="text-[10px] text-text-muted">更新时间</div>
            <div className="font-mono-num text-xs text-text-secondary">{data.update_time}</div>
          </div>
          <div className="text-right">
            <div className="text-[10px] text-text-muted">快照日期</div>
            <div className="font-mono-num text-sm text-bili-blue font-semibold">{data.latest_snapshot_date ?? '-'}</div>
          </div>
        </div>
      )}

      {open &&
        createPortal(
          <div
            ref={menuRef}
            className="fixed z-[9999] w-44 rounded-lg border border-border-default bg-bg-card py-1 shadow-lg shadow-black/40 animate-fade-in"
            style={{ top: pos.top, left: pos.left }}
          >
            <button
              onClick={() => { setSelectedSnapshotDate(null); setOpen(false) }}
              className={`w-full px-3 py-1.5 text-left text-xs transition-colors hover:bg-bg-elevated ${!selectedSnapshotDate ? 'text-bili-blue' : 'text-text-muted'}`}
            >
              最新快照
            </button>
            <div className="mx-2 my-1 border-t border-border-default" />
            {dates.map((d) => (
              <button
                key={d.snapshot_date}
                onClick={() => { setSelectedSnapshotDate(d.snapshot_date); setOpen(false) }}
                className={`w-full px-3 py-1.5 text-left text-xs transition-colors hover:bg-bg-elevated ${selectedSnapshotDate === d.snapshot_date ? 'text-bili-blue' : 'text-text-secondary'}`}
              >
                {d.snapshot_date}
              </button>
            ))}
          </div>,
          document.body
        )}
    </header>
  )
}
