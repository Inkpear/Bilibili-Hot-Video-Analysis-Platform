import { useDashboard } from '../context/DashboardContext';

export default function Header() {
  const { state, setSnapshotDate } = useDashboard();

  return (
    <header className="flex items-center justify-between px-6 py-3 bg-gray-800">
      <div className="flex items-center gap-3">
        <h1 className="text-xl font-bold bg-gradient-to-r from-pink-500 to-blue-500 bg-clip-text text-transparent">
          B站热门视频分析大屏
        </h1>
      </div>

      <div className="flex items-center gap-4">
        <input
          type="date"
          value={state.snapshotDate || ''}
          onChange={(e) => setSnapshotDate(e.target.value)}
          className="bg-gray-700 text-white px-3 py-1 rounded"
        />
      </div>
    </header>
  );
}
