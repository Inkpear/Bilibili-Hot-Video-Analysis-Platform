import ErrorBoundary from './components/common/ErrorBoundary'
import Header from './components/Header'
import KPICards from './components/KPICards'
import PartitionBar from './components/PartitionBar'
import WordCloud from './components/WordCloud'
import HotVideoTable from './components/HotVideoTable'
import HotComments from './components/HotComments'
import InteractionRadar from './components/InteractionRadar'
import LifecycleChart from './components/LifecycleChart'

export default function App() {
  return (
    <div className="bg-grid bg-gradient-radial h-screen p-4 flex flex-col gap-3 max-w-[1920px] mx-auto overflow-hidden">
      <div className="animate-fade-in-up">
        <ErrorBoundary>
          <Header />
        </ErrorBoundary>
      </div>

      <div className="animate-fade-in-up delay-100">
        <ErrorBoundary>
          <KPICards />
        </ErrorBoundary>
      </div>

      <div className="flex-1 min-h-0 flex gap-3">
        <div className="w-[24%] min-w-0 flex flex-col gap-3 animate-slide-in-left delay-200">
          <div className="flex-1 min-h-0">
            <ErrorBoundary>
              <PartitionBar />
            </ErrorBoundary>
          </div>
          <div className="flex-1 min-h-0">
            <ErrorBoundary>
              <WordCloud />
            </ErrorBoundary>
          </div>
        </div>

        <div className="flex-1 min-w-0 flex flex-col gap-3 animate-fade-in-up delay-300">
          <div className="flex-1 min-h-0">
            <ErrorBoundary>
              <HotVideoTable />
            </ErrorBoundary>
          </div>
          <div className="flex-1 min-h-0">
            <ErrorBoundary>
              <HotComments />
            </ErrorBoundary>
          </div>
        </div>

        <div className="w-[28%] min-w-0 flex flex-col gap-3 animate-slide-in-right delay-400">
          <div className="flex-[1] min-h-0">
            <ErrorBoundary>
              <InteractionRadar />
            </ErrorBoundary>
          </div>
          <div className="flex-[2] min-h-0">
            <ErrorBoundary>
              <LifecycleChart />
            </ErrorBoundary>
          </div>
        </div>
      </div>
    </div>
  )
}
