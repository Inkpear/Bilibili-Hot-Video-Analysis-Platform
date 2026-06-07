import { createContext, useContext, useState, ReactNode } from 'react';
import { DashboardState } from '../types';

interface DashboardContextType {
  state: DashboardState;
  setSelectedCategory: (category: string | null) => void;
  setSelectedBvid: (bvid: string | null) => void;
  setSnapshotDate: (date: string | null) => void;
}

const DashboardContext = createContext<DashboardContextType | null>(null);

export function DashboardProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<DashboardState>({
    selectedCategory: null,
    selectedBvid: null,
    snapshotDate: null,
  });

  const setSelectedCategory = (category: string | null) => {
    setState(prev => ({ ...prev, selectedCategory: category }));
  };

  const setSelectedBvid = (bvid: string | null) => {
    setState(prev => ({ ...prev, selectedBvid: bvid }));
  };

  const setSnapshotDate = (date: string | null) => {
    setState(prev => ({ ...prev, snapshotDate: date }));
  };

  return (
    <DashboardContext.Provider value={{
      state,
      setSelectedCategory,
      setSelectedBvid,
      setSnapshotDate,
    }}>
      {children}
    </DashboardContext.Provider>
  );
}

export function useDashboard() {
  const context = useContext(DashboardContext);
  if (!context) {
    throw new Error('useDashboard must be used within DashboardProvider');
  }
  return context;
}
