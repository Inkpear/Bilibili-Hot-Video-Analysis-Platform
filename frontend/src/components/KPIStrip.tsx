import { useApi } from '../hooks/useApi';
import { fetchOverview } from '../api';
import { useDashboard } from '../context/DashboardContext';

export default function KPIStrip() {
  const { state } = useDashboard();
  const { data } = useApi(() => fetchOverview(state.snapshotDate || undefined), [state.snapshotDate]);

  if (!data) return <div className="h-24 bg-gray-800 animate-pulse" />;

  const items = [
    { label: '总播放量', value: formatNumber(data.total_views), color: 'text-blue-400' },
    { label: '总点赞数', value: formatNumber(data.total_likes), color: 'text-pink-400' },
    { label: '活跃分区', value: `${data.partition_count}个`, color: 'text-white' },
    { label: '今日热点', value: data.hot_title, color: 'text-yellow-400' },
    { label: '新增视频', value: `${data.new_video_count}个`, color: 'text-green-400' },
  ];

  return (
    <div className="grid grid-cols-5 gap-4 px-6 py-4 bg-gray-800/50">
      {items.map((item, i) => (
        <div key={i} className="text-center">
          <div className="text-sm text-gray-400">{item.label}</div>
          <div className={`text-2xl font-bold ${item.color}`}>{item.value}</div>
        </div>
      ))}
    </div>
  );
}

function formatNumber(num: number): string {
  if (num >= 100000000) return (num / 100000000).toFixed(1) + '亿';
  if (num >= 10000) return (num / 10000).toFixed(1) + '万';
  return num.toString();
}
