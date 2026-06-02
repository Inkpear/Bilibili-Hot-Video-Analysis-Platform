export interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

export interface OverviewData {
  video_count: number;
  category_count: number;
  up_count: number;
  total_views: number;
  total_interactions: number;
  latest_snapshot_date: string | null;
  update_time: string;
}

export interface HotVideoItem {
  bvid: string;
  title: string;
  category: string;
  up_name: string;
  view_count: number;
  like_count: number;
  coin_count: number;
  favorite_count: number;
  reply_count: number;
  share_count: number;
  rank_position: number;
  heat_score: number;
}

export interface HotVideosData {
  total: number;
  videos: HotVideoItem[];
}

export type HotVideosSort = 'heat' | 'views' | 'likes' | 'coins';

export interface PartitionCompareItem {
  category: string;
  hot_video_count: number;
  avg_views: number;
  avg_like_rate: number;
  avg_coin_rate: number;
  avg_fav_rate: number;
  top_view_count: number;
  total_views: number;
  up_count: number;
}

export interface PartitionCompareData {
  partitions: PartitionCompareItem[];
}

export interface LifecyclePoint {
  snapshot_date: string;
  snapshot_time: string;
  view_count: number;
  like_count: number;
  rank_position: number;
  rank_source: string;
  rank_name: string;
  view_growth: number;
  rank_change: number;
}

export interface LifecycleData {
  bvid: string;
  title: string;
  category: string;
  timeline: LifecyclePoint[];
}

export interface LifecycleTopData {
  videos: LifecycleData[];
}

export interface PartitionDetailVideo {
  bvid: string;
  title: string;
  view_count: number;
  heat_score: number;
  rank_position: number;
}

export interface PartitionDetailUp {
  up_id: string;
  up_name: string;
  follower_count: number;
  hot_video_count: number;
  total_views: number;
}

export interface PartitionDetailData {
  category: string;
  hot_video_count: number;
  avg_views: number;
  videos: PartitionDetailVideo[];
  top_ups: PartitionDetailUp[];
}

export interface WordCloudItem {
  name: string;
  value: number;
}

export type WordCloudSource = 'title' | 'comment' | 'danmaku';

export interface WordCloudData {
  source: string;
  category: string;
  words: WordCloudItem[];
}

export interface HotCommentItem {
  bvid: string;
  title: string;
  category: string;
  up_name: string;
  user_name: string;
  content: string;
  like_count: number;
  reply_count: number;
  comment_time: string | null;
}

export interface HotCommentsData {
  comments: HotCommentItem[];
}

export interface InteractionData {
  bvid: string;
  view_count: number;
  like_count: number;
  coin_count: number;
  favorite_count: number;
  reply_count: number;
  share_count: number;
  like_rate: number;
  coin_rate: number;
  fav_rate: number;
  reply_rate: number;
  share_rate: number;
}
