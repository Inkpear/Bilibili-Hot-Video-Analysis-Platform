import { useDashboard } from '../context/DashboardContext';

export default function VideoDetailPanel() {
  const { state, setSelectedBvid } = useDashboard();
  const { selectedBvid } = state;

  if (!selectedBvid) {
    return (
      <div className="bg-gray-800 rounded-lg p-4 h-full flex items-center justify-center">
        <p className="text-gray-500 text-sm">点击左侧视频列表查看详情</p>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm text-gray-400">视频详情</h3>
        <button
          onClick={() => setSelectedBvid(null)}
          className="text-xs text-gray-500 hover:text-white transition-colors"
        >
          关闭 ✕
        </button>
      </div>
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500 bg-gray-700 px-2 py-0.5 rounded font-mono">
            {selectedBvid}
          </span>
        </div>
        <div className="space-y-2">
          {['标题', 'UP主', '播放量', '点赞数', '弹幕数', '评论数', '收藏数', '分享数', '硬币数'].map(
            (label) => (
              <div key={label} className="flex justify-between items-center">
                <span className="text-sm text-gray-400">{label}</span>
                <div className="h-4 w-16 bg-gray-700 rounded animate-pulse" />
              </div>
            )
          )}
        </div>
      </div>
    </div>
  );
}
