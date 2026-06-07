import { useApi } from '../hooks/useApi';
import { fetchVideos } from '../api';
import { useDashboard } from '../context/DashboardContext';

function formatNum(n: number): string {
  if (n >= 10000) return (n / 10000).toFixed(1) + '万';
  return n.toString();
}

export default function VideoTable() {
  const { state, setSelectedBvid } = useDashboard();

  const { data, loading, error } = useApi(
    () => fetchVideos(state.selectedCategory || undefined, state.snapshotDate || undefined),
    [state.selectedCategory, state.snapshotDate]
  );

  if (loading) {
    return (
      <div className="bg-gray-800 rounded-lg p-4">
        <h3 className="text-sm text-gray-400 mb-3">热门视频</h3>
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="h-12 bg-gray-700/50 rounded animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-gray-800 rounded-lg p-4">
        <h3 className="text-sm text-gray-400 mb-2">热门视频</h3>
        <div className="h-48 flex items-center justify-center text-red-400 text-sm">
          加载失败: {error.message}
        </div>
      </div>
    );
  }

  if (!data || data.length === 0) {
    return (
      <div className="bg-gray-800 rounded-lg p-4">
        <h3 className="text-sm text-gray-400 mb-2">热门视频</h3>
        <div className="h-48 flex items-center justify-center text-gray-500 text-sm">
          暂无视频数据
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-3">
        热门视频
        {state.selectedCategory && (
          <span className="ml-2 text-pink-400">· {state.selectedCategory}</span>
        )}
      </h3>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-gray-500 border-b border-gray-700">
              <th className="text-left py-2 pr-2 w-24">BV号</th>
              <th className="text-left py-2 px-2">标题</th>
              <th className="text-left py-2 px-2 w-20">UP主</th>
              <th className="text-right py-2 pl-2 w-20">播放</th>
            </tr>
          </thead>
          <tbody>
            {data.map((video) => (
              <tr
                key={video.bvid}
                onClick={() => setSelectedBvid(video.bvid)}
                className={`border-b border-gray-700/50 cursor-pointer transition-colors hover:bg-gray-700/30 ${
                  state.selectedBvid === video.bvid ? 'bg-pink-500/10' : ''
                }`}
              >
                <td className="py-2 pr-2 text-gray-400 font-mono text-xs">{video.bvid}</td>
                <td className="py-2 px-2 text-white truncate max-w-xs" title={video.title}>
                  {video.title}
                </td>
                <td className="py-2 px-2 text-gray-400">{video.up_name}</td>
                <td className="py-2 pl-2 text-right text-blue-400">{formatNum(video.view_count)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
