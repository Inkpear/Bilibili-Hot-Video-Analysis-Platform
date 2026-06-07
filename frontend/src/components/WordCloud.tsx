import { useState } from 'react';
import { useApi } from '../hooks/useApi';
import { fetchWordCloud } from '../api';
import { useDashboard } from '../context/DashboardContext';

const TABS = [
  { key: 'title', label: '标题' },
  { key: 'tag', label: '标签' },
  { key: 'comment', label: '评论' },
];

export default function WordCloud() {
  const { state } = useDashboard();
  const [activeTab, setActiveTab] = useState('title');

  const { data, loading } = useApi(
    () => fetchWordCloud(activeTab, state.selectedCategory || undefined, state.snapshotDate || undefined),
    [activeTab, state.selectedCategory, state.snapshotDate]
  );

  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm text-gray-400">词云</h3>
        <div className="flex gap-1">
          {TABS.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`px-2 py-0.5 text-xs rounded transition-colors ${
                activeTab === tab.key
                  ? 'bg-pink-500 text-white'
                  : 'bg-gray-700 text-gray-400 hover:text-white'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>
      {loading ? (
        <div className="h-64 bg-gray-700/50 rounded animate-pulse" />
      ) : !data || data.length === 0 ? (
        <div className="h-64 flex items-center justify-center text-gray-500 text-sm">
          暂无词云数据
        </div>
      ) : (
        <div className="h-64 flex flex-wrap items-center justify-center gap-2 p-2 overflow-hidden">
          {data.map((item, i) => (
            <span
              key={i}
              style={{
                fontSize: `${12 + (item.count / Math.max(...data.map(d => d.count))) * 24}px`,
                opacity: 0.5 + (item.count / Math.max(...data.map(d => d.count))) * 0.5,
              }}
              className="text-pink-300 hover:text-pink-200 cursor-default transition-colors"
              title={`${item.word}: ${item.count}`}
            >
              {item.word}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
