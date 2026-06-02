import axios from 'axios';
import type {
  ApiResponse,
  OverviewData,
  HotVideosData,
  HotVideosSort,
  PartitionCompareData,
  LifecycleData,
  LifecycleTopData,
  PartitionDetailData,
  WordCloudData,
  WordCloudSource,
  HotCommentsData,
  InteractionData,
  InteractionMaxData,
} from '../types';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '';

function getToken(): string {
  const stored = localStorage.getItem('bili_token');
  if (stored) return stored;

  const envToken = import.meta.env.VITE_API_TOKEN || '';
  if (envToken) {
    localStorage.setItem('bili_token', envToken);
    return envToken;
  }

  const input = window.prompt('请输入 API Token：');
  const token = input || '';
  localStorage.setItem('bili_token', token);
  return token;
}

const client = axios.create({
  baseURL: API_BASE,
  timeout: 15000,
});

client.interceptors.request.use((config) => {
  const token = getToken();
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

client.interceptors.response.use(
  (res) => res,
  (err) => {
    if (axios.isAxiosError(err) && err.response?.status === 401) {
      localStorage.removeItem('bili_token');
      window.alert('Token 已过期或无效，请刷新页面重新输入');
    }
    return Promise.reject(err);
  }
);

async function get<T>(url: string, params?: Record<string, unknown>): Promise<T> {
  const res = await client.get<ApiResponse<T>>(url, { params });
  if (res.data.code !== 0) {
    throw new Error(res.data.message);
  }
  return res.data.data;
}

export function fetchOverview() {
  return get<OverviewData>('/api/overview');
}

export function fetchHotVideos(sort: HotVideosSort = 'heat', limit = 20, offset = 0) {
  return get<HotVideosData>('/api/hot-videos', { sort, limit, offset });
}

export function fetchPartitionCompare() {
  return get<PartitionCompareData>('/api/partition-compare');
}

export function fetchLifecycle(bvid: string) {
  return get<LifecycleData>('/api/lifecycle', { bvid });
}

export function fetchLifecycleTop(limit = 5) {
  return get<LifecycleTopData>('/api/lifecycle/top', { limit });
}

export function fetchPartitionDetail(category: string) {
  return get<PartitionDetailData>('/api/partition-detail', { category });
}

export function fetchWordCloud(source: WordCloudSource = 'title', category?: string, bvid?: string) {
  const params: Record<string, string> = { source };
  if (category) params.category = category;
  if (bvid) params.bvid = bvid;
  return get<WordCloudData>('/api/word-cloud', params);
}

export function fetchHotComments(category?: string, bvid?: string, limit = 20) {
  const params: Record<string, string | number> = { limit };
  if (category) params.category = category;
  if (bvid) params.bvid = bvid;
  return get<HotCommentsData>('/api/hot-comments', params);
}

export function fetchInteraction(bvid: string) {
  return get<InteractionData>('/api/interaction', { bvid });
}

export function fetchInteractionMax() {
  return get<InteractionMaxData>('/api/interaction-max');
}
