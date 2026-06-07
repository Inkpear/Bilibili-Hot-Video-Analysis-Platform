import { useDashboard } from '../context/DashboardContext';
import ReactECharts from 'echarts-for-react';

export default function TrendChart() {
  const { state } = useDashboard();

  const option = {
    tooltip: { trigger: 'axis' as const },
    legend: { data: ['播放量', '点赞数'], textStyle: { color: '#9CA3AF' } },
    grid: { left: 50, right: 20, top: 30, bottom: 40 },
    xAxis: {
      type: 'category' as const,
      data: [],
      axisLabel: { color: '#9CA3AF' },
    },
    yAxis: {
      type: 'value' as const,
      axisLabel: { color: '#9CA3AF' },
    },
    series: [
      { name: '播放量', type: 'line' as const, data: [], smooth: true },
      { name: '点赞数', type: 'line' as const, data: [], smooth: true },
    ],
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-2">趋势折线图</h3>
      <ReactECharts option={option} style={{ height: '250px' }} />
    </div>
  );
}
