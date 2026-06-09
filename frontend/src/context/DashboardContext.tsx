import { createContext, useContext, useState, useCallback, useRef } from 'react';
import type { ReactNode } from 'react';
import type { WordCloudSource, HotVideosSort } from '../types';

export interface CarouselState {
  on: boolean;
  wordCloud: {
    commentShown: boolean;
    danmakuShown: boolean;
    currentSource: WordCloudSource;
  };
  commentsEnded: boolean;
}

interface DashboardState {
  selectedBvid: string | null;
  selectedCategory: string | null;
  selectedWord: string | null;
  selectedSnapshotDate: string | null;
  sort: HotVideosSort;
  wordCloudSource: WordCloudSource;
  carousel: CarouselState;
  setSelectedBvid: (bvid: string | null) => void;
  setSelectedCategory: (category: string | null) => void;
  setSelectedWord: (word: string | null) => void;
  setSelectedSnapshotDate: (date: string | null) => void;
  setSort: (sort: HotVideosSort) => void;
  setWordCloudSource: (source: WordCloudSource) => void;
  startCarousel: () => void;
  stopCarousel: () => void;
  markWordCloudShown: (source: WordCloudSource) => void;
  markCommentsEnded: () => void;
  advanceCarousel: () => void;
  carouselAdvanceRef: React.MutableRefObject<(() => void) | null>;
}

const CAROUSEL_INITIAL: CarouselState = {
  on: false,
  wordCloud: { commentShown: false, danmakuShown: false, currentSource: 'comment' },
  commentsEnded: false,
};

const DashboardContext = createContext<DashboardState | null>(null);

export function DashboardProvider({ children }: { children: ReactNode }) {
  const [selectedBvid, setSelectedBvid] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedWord, setSelectedWord] = useState<string | null>(null);
  const [selectedSnapshotDate, setSelectedSnapshotDate] = useState<string | null>(null);
  const [sort, setSort] = useState<HotVideosSort>('heat');
  const [wordCloudSource, setWordCloudSource] = useState<WordCloudSource>('title');
  const [carousel, setCarousel] = useState<CarouselState>(CAROUSEL_INITIAL);
  const carouselAdvanceRef = useRef<(() => void) | null>(null);

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

  const startCarousel = useCallback(() => {
    setCarousel({
      on: true,
      wordCloud: { commentShown: false, danmakuShown: false, currentSource: 'comment' },
      commentsEnded: false,
    });
    setWordCloudSource('comment');
  }, [setWordCloudSource]);

  const stopCarousel = useCallback(() => {
    setCarousel(CAROUSEL_INITIAL);
  }, []);

  const markWordCloudShown = useCallback((source: WordCloudSource) => {
    setCarousel((prev) => ({
      ...prev,
      wordCloud: {
        ...prev.wordCloud,
        commentShown: prev.wordCloud.commentShown || source === 'comment',
        danmakuShown: prev.wordCloud.danmakuShown || source === 'danmaku',
      },
    }));
  }, []);

  const markCommentsEnded = useCallback(() => {
    setCarousel((prev) => ({ ...prev, commentsEnded: true }));
  }, []);

  const advanceCarousel = useCallback(() => {
    if (carouselAdvanceRef.current) {
      carouselAdvanceRef.current();
    }
    setCarousel({
      on: true,
      wordCloud: { commentShown: false, danmakuShown: false, currentSource: 'comment' },
      commentsEnded: false,
    });
    setWordCloudSource('comment');
  }, [setWordCloudSource]);

  return (
    <DashboardContext.Provider
      value={{
        selectedBvid,
        selectedCategory,
        selectedWord,
        selectedSnapshotDate,
        sort,
        wordCloudSource,
        carousel,
        setSelectedBvid: handleSetSelectedBvid,
        setSelectedCategory: handleSetSelectedCategory,
        setSelectedWord: handleSetSelectedWord,
        setSelectedSnapshotDate,
        setSort,
        setWordCloudSource: handleSetWordCloudSource,
        startCarousel,
        stopCarousel,
        markWordCloudShown,
        markCommentsEnded,
        advanceCarousel,
        carouselAdvanceRef,
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
