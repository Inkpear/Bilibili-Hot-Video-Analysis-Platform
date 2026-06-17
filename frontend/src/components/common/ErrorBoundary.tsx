import { Component } from 'react'
import type { ErrorInfo, ReactNode } from 'react'

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Dashboard error:', error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback || (
          <div className="card p-6 flex flex-col items-center justify-center gap-3 text-center">
            <div className="w-12 h-12 rounded-full bg-negative/20 flex items-center justify-center">
              <span className="text-negative text-xl">!</span>
            </div>
            <div>
              <h3 className="text-text-primary font-medium mb-1">组件渲染出错</h3>
              <p className="text-text-muted text-sm">{this.state.error?.message}</p>
            </div>
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="px-3 py-1.5 text-sm rounded bg-bili-blue text-white hover:bg-bili-blue-light transition-colors"
            >
              重试
            </button>
          </div>
        )
      )
    }

    return this.props.children
  }
}
