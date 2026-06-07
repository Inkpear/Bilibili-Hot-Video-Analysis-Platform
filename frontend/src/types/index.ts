export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

export interface OverviewData {
  total_views: number;
  total_likes: number;
  partition_count: number;
  hot_title: string;
  new_video_count: number;
}

export interface PartitionHeatmapData {
  categories: string[];
  dates: string[];
  values: number[][];
}

export interface VideoListItem {
  bvid: string;
  title: string;
  up_name: string;
  category: string;
  view_count: number;
  like_count: number;
}

export interface WordCloudItem {
  word: string;
  count: number;
}

export interface DashboardState {
  selectedCategory: string | null;
  selectedBvid: string | null;
  snapshotDate: string | null;
}
