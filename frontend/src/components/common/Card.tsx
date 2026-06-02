import type { ReactNode, CSSProperties } from 'react';

interface CardProps {
  title?: string;
  extra?: ReactNode;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  span?: number;
  rowSpan?: number;
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
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border-default shrink-0">
          {title && (
            <h3 className="font-display text-xs tracking-wider text-text-secondary uppercase flex items-center gap-2">
              <span className="w-1 h-3 bg-bili-blue rounded-full" />
              {title}
            </h3>
          )}
          {extra && <div className="flex items-center gap-2">{extra}</div>}
        </div>
      )}
      <div className="flex-1 p-3 min-h-0">{children}</div>
    </div>
  );
}
