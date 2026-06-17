import type { ReactNode, CSSProperties } from 'react'

interface CardProps {
  title?: string
  extra?: ReactNode
  children: ReactNode
  className?: string
  style?: CSSProperties
  span?: number
  rowSpan?: number
}

export default function Card({ title, extra, children, className = '', style, span, rowSpan }: CardProps) {
  return (
    <div
      className={`card flex flex-col animate-fade-in-up ${className}`}
      style={{
        ...style,
        gridColumn: span ? `span ${span}` : undefined,
        gridRow: rowSpan ? `span ${rowSpan}` : undefined,
      }}
    >
      {(title || extra) && (
        <div className="flex shrink-0 items-center justify-between border-b border-border-default bg-[linear-gradient(180deg,rgba(255,255,255,0.03),rgba(255,255,255,0.0))] px-4 py-2.5">
          {title && (
            <h3 className="flex items-center gap-2 font-display text-xs uppercase tracking-wider text-text-secondary">
              <span className="h-3 w-1 rounded-full bg-bili-blue shadow-[0_0_10px_rgba(0,161,214,0.55)]" />
              {title}
            </h3>
          )}
          {extra && <div className="flex max-w-[72%] items-center justify-end gap-2 text-right">{extra}</div>}
        </div>
      )}
      <div className="flex-1 min-h-0 p-3">{children}</div>
    </div>
  )
}
