import { useApi } from '../hooks/useApi';
import { fetchPartitionHeatmap } from '../api';
import { useDashboard } from '../context/DashboardContext';
import ReactECharts from 'echarts-for-react';

export default function PartitionHeatmap() {
  const { state, setSelectedCategory } = useDashboard();
  const { data } = useApi(
    () => fetchPartitionHeatmap(state.snapshotDate || undefined),
    [state.snapshotDate]
  );

  if (!data) return <div className="h-64 bg-gray-800 animate-pulse rounded-lg" />;

  const maxVal = Math.max(...data.values.flat());

  const option = {
    tooltip: {
      position: 'top',
      formatter: (params: any) => {
        return `${data.categories[params.value[1]]}<br/>${data.dates[params.value[0]]}<br/>播放量: ${params.value[2]}`;
      },
    },
    grid: {
      left: 80,
      right: 20,
      top: 20,
      bottom: 60,
    },
    xAxis: {
      type: 'category' as const,
      data: data.dates,
      axisLabel: { color: '#9CA3AF' },
    },
    yAxis: {
      type: 'category' as const,
      data: data.categories,
      axisLabel: { color: '#9CA3AF' },
    },
    visualMap: {
      min: 0,
      max: maxVal,
      calculable: true,
      orient: 'horizontal' as const,
      left: 'center',
      bottom: 0,
      inRange: {
        color: ['#1E3A5F', '#00A1D6', '#FB7299'],
      },
      textStyle: { color: '#9CA3AF' },
    },
    series: [
      {
        type: 'heatmap' as const,
        data: data.values
          .map((row, y) => row.map((val, x) => [x, y, val]))
          .flat(),
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
          },
        },
      },
    ],
  };

  const handleClick = (params: any) => {
    const category = data.categories[params.value[1]];
    setSelectedCategory(category === state.selectedCategory ? null : category);
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-2">分区热力图</h3>
      <ReactECharts
        option={option}
        style={{ height: '250px' }}
        onEvents={{ click: handleClick }}
      />
    </div>
  );
}
