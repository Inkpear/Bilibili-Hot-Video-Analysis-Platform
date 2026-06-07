import { useDashboard } from '../context/DashboardContext';
import ReactECharts from 'echarts-for-react';

export default function RadarChart() {
  const { state } = useDashboard();

  const option = {
    radar: {
      indicator: [
        { name: '播放量', max: 100 },
        { name: '点赞数', max: 100 },
        { name: '弹幕数', max: 100 },
        { name: '评论数', max: 100 },
        { name: '分享数', max: 100 },
      ],
      center: ['50%', '55%'],
      radius: '65%',
      axisName: { color: '#9CA3AF' },
      splitArea: {
        areaStyle: { color: ['rgba(255,255,255,0.02)', 'rgba(255,255,255,0.04)'] },
      },
    },
    series: [
      {
        type: 'radar' as const,
        data: [],
        areaStyle: { opacity: 0.2 },
        lineStyle: { color: '#FB7299' },
        itemStyle: { color: '#FB7299' },
      },
    ],
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <h3 className="text-sm text-gray-400 mb-2">
        雷达对比图
        {state.selectedCategory && (
          <span className="ml-2 text-pink-400">· {state.selectedCategory}</span>
        )}
      </h3>
      <ReactECharts option={option} style={{ height: '250px' }} />
    </div>
  );
}
