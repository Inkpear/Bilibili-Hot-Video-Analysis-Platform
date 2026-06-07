import { useDashboard } from '../context/DashboardContext';
import ReactECharts from 'echarts-for-react';

export default function WordTrendChart() {
  const { state } = useDashboard();

  const option = {
    tooltip: { trigger: 'axis' as const },
    legend: { data: [], top: 0, textStyle: { color: '#9CA3AF' } },
    grid: { left: 50, right: 30, top: 30, bottom: 40 },
    xAxis: {
      type: 'category' as const,
      data: [],
      axisLabel: { color: '#9CA3AF' },
    },
    yAxis: {
      type: 'value' as const,
      axisLabel: { color: '#9CA3AF' },
    },
    series: [],
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-2">
        词频趋势
        {state.selectedCategory && (
          <span className="ml-2 text-pink-400">· {state.selectedCategory}</span>
        )}
      </h3>
      <ReactECharts option={option} style={{ height: '250px' }} />
    </div>
  );
}
