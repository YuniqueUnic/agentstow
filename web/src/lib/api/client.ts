import type {
  ArtifactDetailResponse,
  ApiError,
  ImpactAnalysisResponse,
  LinkRecordResponse,
  LinkStatusResponseItem,
  ManifestResponse,
  ProfileDetailResponse,
  RenderResponse,
  WatchStatusResponse,
  WorkspaceSummaryResponse
} from '$lib/types';

export class ApiClientError extends Error {
  readonly status: number;
  readonly details?: ApiError;

  constructor(status: number, message: string, details?: ApiError) {
    super(message);
    this.name = 'ApiClientError';
    this.status = status;
    this.details = details;
  }
}

type QueryValue = string | number | boolean | null | undefined;

const API_BASE = (import.meta.env.VITE_AGENTSTOW_API_BASE ?? '').replace(/\/$/, '');

function buildUrl(path: string, query?: Record<string, QueryValue>): string {
  const url = new URL(`${API_BASE}${path}`, window.location.origin);

  if (!query) {
    return url.toString();
  }

  for (const [key, value] of Object.entries(query)) {
    if (value === null || value === undefined || value === '') {
      continue;
    }

    url.searchParams.set(key, String(value));
  }

  return url.toString();
}

function isApiError(payload: unknown): payload is ApiError {
  return Boolean(
    payload &&
      typeof payload === 'object' &&
      'message' in payload &&
      typeof (payload as ApiError).message === 'string'
  );
}

function extractMessage(payload: unknown, status: number): string {
  if (isApiError(payload)) {
    return payload.message;
  }

  if (typeof payload === 'string' && payload.trim()) {
    return payload;
  }

  return `请求失败（${status}）`;
}

async function fetchJson<T>(
  path: string,
  query?: Record<string, QueryValue>,
  init?: RequestInit
): Promise<T> {
  const response = await fetch(buildUrl(path, query), {
    headers: {
      Accept: 'application/json',
      ...(init?.headers ?? {})
    },
    ...init
  });

  const isJson = (response.headers.get('content-type') ?? '').includes('application/json');
  const payload = isJson ? await response.json().catch(() => null) : await response.text();

  if (!response.ok) {
    throw new ApiClientError(
      response.status,
      extractMessage(payload, response.status),
      isApiError(payload) ? payload : undefined
    );
  }

  return payload as T;
}

export function getManifest(): Promise<ManifestResponse> {
  return fetchJson<ManifestResponse>('/api/manifest');
}

export function getLinks(): Promise<LinkRecordResponse[]> {
  return fetchJson<LinkRecordResponse[]>('/api/links');
}

export function getLinkStatus(): Promise<LinkStatusResponseItem[]> {
  return fetchJson<LinkStatusResponseItem[]>('/api/link-status');
}

export function getWatchStatus(): Promise<WatchStatusResponse> {
  return fetchJson<WatchStatusResponse>('/api/watch-status');
}

export function getWorkspaceSummary(): Promise<WorkspaceSummaryResponse> {
  return fetchJson<WorkspaceSummaryResponse>('/api/workspace-summary');
}

export function getArtifactDetail(artifact: string): Promise<ArtifactDetailResponse> {
  return fetchJson<ArtifactDetailResponse>(`/api/artifacts/${encodeURIComponent(artifact)}`);
}

export function getProfileDetail(profile: string): Promise<ProfileDetailResponse> {
  return fetchJson<ProfileDetailResponse>(`/api/profiles/${encodeURIComponent(profile)}`);
}

export function getImpactAnalysis(query: {
  artifact?: string | null;
  profile?: string | null;
}): Promise<ImpactAnalysisResponse> {
  return fetchJson<ImpactAnalysisResponse>('/api/impact', query);
}

export function renderArtifact(artifact: string, profile: string): Promise<RenderResponse> {
  return fetchJson<RenderResponse>('/api/render', { artifact, profile });
}
