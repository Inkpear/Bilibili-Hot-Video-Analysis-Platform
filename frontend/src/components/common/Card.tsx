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
        <div className="flex items-center justify-between px-4 py-3 border-b border-border-default">
          {title && (
            <h3 className="font-display text-sm tracking-wider text-text-secondary uppercase">
              {title}
            </h3>
          )}
          {extra && <div className="flex items-center gap-2">{extra}</div>}
        </div>
      )}
      <div className="flex-1 p-4 min-h-0">{children}</div>
    </div>
  );
}
