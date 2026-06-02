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
    <div className="bg-grid min-h-screen p-4 flex flex-col gap-4 max-w-[1920px] mx-auto">
      <ErrorBoundary>
        <Header />
      </ErrorBoundary>

      <ErrorBoundary>
        <KPICards />
      </ErrorBoundary>

      <div className="flex-1 grid grid-cols-12 gap-4 min-h-[600px]">
        <div className="col-span-3 flex flex-col gap-4 min-h-0">
          <ErrorBoundary>
            <PartitionBar />
          </ErrorBoundary>
          <ErrorBoundary>
            <WordCloud />
          </ErrorBoundary>
        </div>

        <div className="col-span-5 flex flex-col gap-4 min-h-0">
          <ErrorBoundary>
            <HotVideoTable />
          </ErrorBoundary>
          <ErrorBoundary>
            <HotComments />
          </ErrorBoundary>
        </div>

        <div className="col-span-4 flex flex-col gap-4 min-h-0">
          <ErrorBoundary>
            <InteractionRadar />
          </ErrorBoundary>
          <ErrorBoundary>
            <LifecycleChart />
          </ErrorBoundary>
        </div>
      </div>
    </div>
  )
}
