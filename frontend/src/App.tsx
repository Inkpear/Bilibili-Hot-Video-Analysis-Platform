import { useState } from 'react'
import ErrorBoundary from './components/common/ErrorBoundary'
import Header from './components/Header'
import KPICards from './components/KPICards'
import PartitionBar from './components/PartitionBar'
import WordCloud from './components/WordCloud'
import HotVideoTable from './components/HotVideoTable'
import HotComments from './components/HotComments'
import InteractionRadar from './components/InteractionRadar'
import LifecycleChart from './components/LifecycleChart'
import PartitionMetricMatrix from './components/PartitionMetricMatrix'
import PartitionDetailPanel from './components/PartitionDetailPanel'
import WordTrendChart from './components/WordTrendChart'

export default function App() {
  const [screen, setScreen] = useState<'overview' | 'analysis'>('overview')

  return (
    <div className="bg-grid bg-gradient-radial relative h-screen overflow-hidden">
      <section data-screen="overview" className={screen === 'overview' ? 'block h-full' : 'hidden'}>
        <div className="mx-auto flex h-full max-w-[1920px] flex-col gap-3 p-4">
          <div className="shrink-0 animate-fade-in-up">
            <ErrorBoundary>
              <Header screen={screen} onScreenChange={setScreen} />
            </ErrorBoundary>
          </div>

          <div className="shrink-0 animate-fade-in-up delay-100">
            <ErrorBoundary>
              <KPICards />
            </ErrorBoundary>
          </div>

          <div className="grid min-h-0 flex-1 grid-cols-12 grid-rows-2 gap-3">
            <div className="col-span-3 min-h-0 animate-slide-in-left delay-200">
              <ErrorBoundary>
                <PartitionBar />
              </ErrorBoundary>
            </div>

            <div className="col-span-5 min-h-0 animate-fade-in-up delay-200">
              <ErrorBoundary>
                <HotVideoTable />
              </ErrorBoundary>
            </div>

            <div className="col-span-4 min-h-0 animate-slide-in-right delay-200">
              <ErrorBoundary>
                <LifecycleChart />
              </ErrorBoundary>
            </div>

            <div className="col-span-3 min-h-0 animate-slide-in-left delay-300">
              <ErrorBoundary>
                <WordCloud />
              </ErrorBoundary>
            </div>

            <div className="col-span-5 min-h-0 animate-fade-in-up delay-300">
              <ErrorBoundary>
                <HotComments />
              </ErrorBoundary>
            </div>

            <div className="col-span-4 min-h-0 animate-slide-in-right delay-300">
              <ErrorBoundary>
                <InteractionRadar />
              </ErrorBoundary>
            </div>
          </div>
        </div>
      </section>

      <section data-screen="analysis" className={screen === 'analysis' ? 'block h-full' : 'hidden'}>
        <div className="mx-auto flex h-full max-w-[1920px] flex-col gap-3 p-4">
          <div className="shrink-0 animate-fade-in-up">
            <ErrorBoundary>
              <Header screen={screen} onScreenChange={setScreen} />
            </ErrorBoundary>
          </div>

          <div className="grid min-h-0 flex-1 grid-cols-12 grid-rows-[0.95fr_1.05fr] gap-3">
            <div className="col-span-7 min-h-0 animate-slide-in-left delay-200">
              <ErrorBoundary>
                <PartitionMetricMatrix />
              </ErrorBoundary>
            </div>

            <div className="col-span-5 min-h-0 animate-slide-in-right delay-200">
              <ErrorBoundary>
                <WordTrendChart />
              </ErrorBoundary>
            </div>

            <div className="col-span-12 min-h-0 animate-fade-in-up delay-300">
              <ErrorBoundary>
                <PartitionDetailPanel />
              </ErrorBoundary>
            </div>
          </div>
        </div>
      </section>
    </div>
  )
}
