import { DashboardProvider, useDashboard } from './context/DashboardContext';
import Header from './components/Header';
import KPIStrip from './components/KPIStrip';
import PartitionHeatmap from './components/PartitionHeatmap';
import TrendChart from './components/TrendChart';
import WordCloud from './components/WordCloud';
import VideoTable from './components/VideoTable';
import RadarChart from './components/RadarChart';
import WordTrendChart from './components/WordTrendChart';
import VideoDetailPanel from './components/VideoDetailPanel';

function DashboardContent() {
  const { state } = useDashboard();

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      <Header />
      <KPIStrip />

      <div className="grid grid-cols-3 gap-4 p-4">
        <PartitionHeatmap />
        <TrendChart />
        <WordCloud />
        <VideoTable />
        <RadarChart />
        <WordTrendChart />
      </div>

      {state.selectedBvid && <VideoDetailPanel />}
    </div>
  );
}

export default function App() {
  return (
    <DashboardProvider>
      <DashboardContent />
    </DashboardProvider>
  );
}
