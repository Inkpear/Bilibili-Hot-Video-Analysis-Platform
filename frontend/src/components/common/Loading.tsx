interface LoadingProps {
  rows?: number;
  className?: string;
}

export default function Loading({ rows = 3, className = '' }: LoadingProps) {
  return (
    <div className={`space-y-3 ${className}`}>
      {Array.from({ length: rows }).map((_, i) => (
        <div
          key={i}
          className="animate-shimmer rounded h-4"
          style={{
            width: `${70 + Math.random() * 30}%`,
            animationDelay: `${i * 0.15}s`,
          }}
        />
      ))}
    </div>
  );
}

export function LoadingCard() {
  return (
    <div className="card p-4 animate-pulse-glow">
      <div className="animate-shimmer rounded h-4 w-1/3 mb-4" />
      <Loading rows={4} />
    </div>
  );
}
