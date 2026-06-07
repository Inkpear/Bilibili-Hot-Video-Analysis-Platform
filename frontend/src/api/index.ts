import { ApiResponse, OverviewData, PartitionHeatmapData, VideoListItem, WordCloudItem } from '../types';

const BASE_URL = '/api';

async function fetchApi<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(path, window.location.origin);
  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value) url.searchParams.append(key, value);
    });
  }
  
  const response = await fetch(url.toString());
  const data: ApiResponse<T> = await response.json();
  
  if (data.code !== 0) {
    throw new Error(data.message);
  }
  
  return data.data;
}

export async function fetchOverview(snapshotDate?: string): Promise<OverviewData> {
  return fetchApi(`${BASE_URL}/overview`, { snapshot_date: snapshotDate || '' });
}

export async function fetchPartitionHeatmap(snapshotDate?: string): Promise<PartitionHeatmapData> {
  return fetchApi(`${BASE_URL}/partition-heatmap`, { snapshot_date: snapshotDate || '' });
}

export async function fetchVideos(category?: string, snapshotDate?: string): Promise<VideoListItem[]> {
  return fetchApi(`${BASE_URL}/videos`, { 
    category: category || '', 
    snapshot_date: snapshotDate || '' 
  });
}

export async function fetchWordCloud(
  sourceType: string, 
  category?: string, 
  snapshotDate?: string
): Promise<WordCloudItem[]> {
  return fetchApi(`${BASE_URL}/word-cloud`, { 
    source_type: sourceType,
    category: category || '', 
    snapshot_date: snapshotDate || '' 
  });
}
