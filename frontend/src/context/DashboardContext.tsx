import { createContext, useContext, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import type { WordCloudSource } from '../types';

interface DashboardState {
  selectedBvid: string | null;
  selectedCategory: string | null;
  selectedWord: string | null;
  wordCloudSource: WordCloudSource;
  carouselOn: boolean;
  setSelectedBvid: (bvid: string | null) => void;
  setSelectedCategory: (category: string | null) => void;
  setSelectedWord: (word: string | null) => void;
  setWordCloudSource: (source: WordCloudSource) => void;
  setCarouselOn: (on: boolean) => void;
}

const DashboardContext = createContext<DashboardState | null>(null);

export function DashboardProvider({ children }: { children: ReactNode }) {
  const [selectedBvid, setSelectedBvid] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedWord, setSelectedWord] = useState<string | null>(null);
  const [wordCloudSource, setWordCloudSource] = useState<WordCloudSource>('title');
  const [carouselOn, setCarouselOn] = useState(false);

  const handleSetSelectedBvid = useCallback((bvid: string | null) => {
    setSelectedBvid(bvid);
  }, []);

  const handleSetSelectedCategory = useCallback((category: string | null) => {
    setSelectedCategory(category);
    setSelectedWord(null);
  }, []);

  const handleSetSelectedWord = useCallback((word: string | null) => {
    setSelectedWord((prev) => (prev === word ? null : word));
  }, []);

  const handleSetWordCloudSource = useCallback((source: WordCloudSource) => {
    setWordCloudSource(source);
  }, []);

  return (
    <DashboardContext.Provider
      value={{
        selectedBvid,
        selectedCategory,
        selectedWord,
        wordCloudSource,
        carouselOn,
        setSelectedBvid: handleSetSelectedBvid,
        setSelectedCategory: handleSetSelectedCategory,
        setSelectedWord: handleSetSelectedWord,
        setWordCloudSource: handleSetWordCloudSource,
        setCarouselOn,
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
