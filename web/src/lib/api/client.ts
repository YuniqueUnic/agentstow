import type {
  ArtifactDetailResponse,
  ArtifactSourceResponse,
  ApiError,
  EnvEmitRequest,
  EnvEmitResponse,
  ImpactAnalysisResponse,
  LinkRecordResponse,
  LinkApplyRequest,
  LinkOperationResponse,
  LinkPlanRequest,
  LinkRepairRequest,
  LinkStatusResponseItem,
  ManifestResponse,
  ManifestSourceResponse,
  ManifestSourceUpdateRequest,
  ProfileDetailResponse,
  RenderResponse,
  ScriptRunRequest,
  ScriptRunResponse,
  WatchStatusResponse,
  WorkspaceInitRequest,
  WorkspaceInitResponse,
  WorkspaceSelectResponse,
  WorkspaceStateResponse,
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

export function getManifestSource(): Promise<ManifestSourceResponse> {
  return fetchJson<ManifestSourceResponse>('/api/manifest/source');
}

export function updateManifestSource(
  request: ManifestSourceUpdateRequest
): Promise<ManifestSourceResponse> {
  return fetchJson<ManifestSourceResponse>('/api/manifest/source', undefined, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  });
}

export function getWorkspaceState(): Promise<WorkspaceStateResponse> {
  return fetchJson<WorkspaceStateResponse>('/api/workspace');
}

export function selectWorkspace(workspace_root: string): Promise<WorkspaceSelectResponse> {
  return fetchJson<WorkspaceSelectResponse>('/api/workspace', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ workspace_root })
  });
}

export function initWorkspace(request: WorkspaceInitRequest): Promise<WorkspaceInitResponse> {
  return fetchJson<WorkspaceInitResponse>('/api/workspace/init', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  });
}

export function getLinks(): Promise<LinkRecordResponse[]> {
  return fetchJson<LinkRecordResponse[]>('/api/links');
}

export function getLinkStatus(): Promise<LinkStatusResponseItem[]> {
  return fetchJson<LinkStatusResponseItem[]>('/api/link-status');
}

export function planLinks(request: LinkPlanRequest): Promise<LinkOperationResponse> {
  return fetchJson<LinkOperationResponse>('/api/links/plan', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  });
}

export function applyLinks(request: LinkApplyRequest): Promise<LinkOperationResponse> {
  return fetchJson<LinkOperationResponse>('/api/links/apply', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  });
}

export function repairLinks(request: LinkRepairRequest): Promise<LinkOperationResponse> {
  return fetchJson<LinkOperationResponse>('/api/links/repair', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  });
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

export function getArtifactSource(artifact: string): Promise<ArtifactSourceResponse> {
  return fetchJson<ArtifactSourceResponse>(`/api/artifacts/${encodeURIComponent(artifact)}/source`);
}

export function updateArtifactSource(
  artifact: string,
  content: string
): Promise<ArtifactSourceResponse> {
  return fetchJson<ArtifactSourceResponse>(
    `/api/artifacts/${encodeURIComponent(artifact)}/source`,
    undefined,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ content })
    }
  );
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

export function emitEnv(request: EnvEmitRequest): Promise<EnvEmitResponse> {
  return fetchJson<EnvEmitResponse>('/api/env/emit', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(request)
  });
}

export function runScript(scriptId: string, request: ScriptRunRequest): Promise<ScriptRunResponse> {
  return fetchJson<ScriptRunResponse>(
    `/api/scripts/${encodeURIComponent(scriptId)}/run`,
    undefined,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(request)
    }
  );
}
