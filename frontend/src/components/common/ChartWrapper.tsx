import { useRef, useEffect, useState } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';

interface ChartWrapperProps {
  option: EChartsOption;
  className?: string;
  loading?: boolean;
  onEvents?: Record<string, (params: unknown) => void>;
}

const BILIBILI_THEME: EChartsOption = {
  color: ['#00A1D6', '#FB7299', '#39D353', '#F0883E', '#A371F7', '#79C0FF', '#FF9CB8', '#23C4F4'],
  backgroundColor: 'transparent',
  textStyle: {
    color: '#8B949E',
    fontFamily: 'JetBrains Mono, Noto Sans SC, sans-serif',
  },
  title: {
    textStyle: { color: '#E6EDF3', fontWeight: 600 },
  },
  legend: {
    textStyle: { color: '#8B949E' },
  },
  tooltip: {
    backgroundColor: '#21262D',
    borderColor: '#30363D',
    textStyle: { color: '#E6EDF3', fontSize: 12 },
  },
  categoryAxis: {
    axisLine: { lineStyle: { color: '#30363D' } },
    axisTick: { lineStyle: { color: '#30363D' } },
    axisLabel: { color: '#8B949E' },
    splitLine: { lineStyle: { color: '#21262D' } },
  },
  valueAxis: {
    axisLine: { lineStyle: { color: '#30363D' } },
    axisTick: { lineStyle: { color: '#30363D' } },
    axisLabel: { color: '#8B949E' },
    splitLine: { lineStyle: { color: '#21262D', type: 'dashed' } },
  },
};

export default function ChartWrapper({ option, className = '', loading = false, onEvents }: ChartWrapperProps) {
  const chartRef = useRef<ReactECharts>(null);
  const [mergedOption, setMergedOption] = useState<EChartsOption>(option);

  useEffect(() => {
    setMergedOption({ ...BILIBILI_THEME, ...option });
  }, [option]);

  useEffect(() => {
    const instance = chartRef.current?.getEchartsInstance();
    if (!instance) return;

    const handleResize = () => instance.resize();
    const observer = new ResizeObserver(handleResize);
    const el = instance.getDom();
    if (el) observer.observe(el.parentElement || el);

    return () => observer.disconnect();
  }, []);

  return (
    <div className={`relative w-full h-full min-h-0 ${className}`}>
      <ReactECharts
        ref={chartRef}
        option={mergedOption}
        style={{ width: '100%', height: '100%' }}
        opts={{ renderer: 'canvas' }}
        onEvents={onEvents}
      />
      {loading && (
        <div className="absolute inset-0 flex items-center justify-center bg-bg-card/80 backdrop-blur-sm">
          <div className="w-8 h-8 border-2 border-bili-blue border-t-transparent rounded-full animate-spin" />
        </div>
      )}
    </div>
  );
}
