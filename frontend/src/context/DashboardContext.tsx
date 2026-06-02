import { createContext, useContext, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import type { WordCloudSource } from '../types';

interface DashboardState {
  selectedBvid: string | null;
  selectedCategory: string | null;
  wordCloudSource: WordCloudSource;
  setSelectedBvid: (bvid: string | null) => void;
  setSelectedCategory: (category: string | null) => void;
  setWordCloudSource: (source: WordCloudSource) => void;
}

const DashboardContext = createContext<DashboardState | null>(null);

export function DashboardProvider({ children }: { children: ReactNode }) {
  const [selectedBvid, setSelectedBvid] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [wordCloudSource, setWordCloudSource] = useState<WordCloudSource>('title');

  const handleSetSelectedBvid = useCallback((bvid: string | null) => {
    setSelectedBvid(bvid);
  }, []);

  const handleSetSelectedCategory = useCallback((category: string | null) => {
    setSelectedCategory(category);
  }, []);

  const handleSetWordCloudSource = useCallback((source: WordCloudSource) => {
    setWordCloudSource(source);
  }, []);

  return (
    <DashboardContext.Provider
      value={{
        selectedBvid,
        selectedCategory,
        wordCloudSource,
        setSelectedBvid: handleSetSelectedBvid,
        setSelectedCategory: handleSetSelectedCategory,
        setWordCloudSource: handleSetWordCloudSource,
      }}
    >
      {children}
    </DashboardContext.Provider>
  );
}

export function useDashboard() {
  const ctx = useContext(DashboardContext);
  if (!ctx) throw new Error('useDashboard must be used within DashboardProvider');
  return ctx;
}
