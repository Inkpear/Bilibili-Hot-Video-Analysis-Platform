export function formatNumber(n: number): string {
  if (n >= 1_0000_0000) return (n / 1_0000_0000).toFixed(1) + '亿'
  if (n >= 1_0000) return (n / 1_0000).toFixed(1) + '万'
  return n.toLocaleString()
}

export function formatPercent(n: number): string {
  return (n * 100).toFixed(2) + '%'
}

export function formatCompact(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return n.toString()
}
