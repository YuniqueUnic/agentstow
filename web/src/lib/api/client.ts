import type {
  ArtifactDetailResponse,
  ArtifactGitCompareResponse,
  ArtifactGitHistoryResponse,
  ArtifactGitRollbackRequest,
  ArtifactGitRollbackResponse,
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
  McpRenderResponse,
  McpTestResponse,
  McpValidateResponse,
  ProfileDetailResponse,
  ProfileVarsUpdateRequest,
  RenderResponse,
  ScriptRunRequest,
  ScriptRunResponse,
  WatchStatusResponse,
  WorkspaceGitSummaryResponse,
  WorkspaceInitRequest,
  WorkspaceInitResponse,
  WorkspaceProbeResponse,
  WorkspaceSelectResponse,
  WorkspaceStateResponse,
  WorkspaceSummaryResponse
} from '$lib/types';

type ClipboardLike = {
  writeText?: (text: string) => Promise<void>;
};

type NavigatorWithClipboard = Navigator & {
  clipboard?: ClipboardLike;
};

type WindowWithClipboardState = Window & {
  __agentstowCopiedText__?: string;
};

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

function rememberCopiedText(text: string): void {
  if (typeof window === 'undefined') {
    return;
  }

  (window as WindowWithClipboardState).__agentstowCopiedText__ = text;
}

function legacyCopyText(text: string): boolean {
  if (typeof document === 'undefined' || !document.body) {
    return false;
  }

  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', 'true');
  textarea.style.position = 'fixed';
  textarea.style.top = '0';
  textarea.style.left = '0';
  textarea.style.width = '1px';
  textarea.style.height = '1px';
  textarea.style.opacity = '0';
  textarea.style.pointerEvents = 'none';
  textarea.style.whiteSpace = 'pre';

  const activeElement = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const selection = document.getSelection();
  const ranges = selection
    ? Array.from({ length: selection.rangeCount }, (_, index) => selection.getRangeAt(index).cloneRange())
    : [];

  document.body.append(textarea);

  try {
    textarea.focus({ preventScroll: true });
    textarea.select();
    textarea.setSelectionRange(0, text.length);
    rememberCopiedText(text);

    if (typeof document.execCommand !== 'function') {
      return false;
    }
    return document.execCommand('copy');
  } catch {
    return false;
  } finally {
    textarea.remove();

    if (selection) {
      selection.removeAllRanges();
      for (const range of ranges) {
        selection.addRange(range);
      }
    }

    activeElement?.focus({ preventScroll: true });
  }
}

async function writeTextWithFallback(text: string, originalWriteText?: (text: string) => Promise<void>): Promise<void> {
  const normalized = String(text);
  let originalError: unknown;

  if (originalWriteText) {
    try {
      await originalWriteText(normalized);
      rememberCopiedText(normalized);
      return;
    } catch (error) {
      originalError = error;
    }
  }

  if (legacyCopyText(normalized)) {
    rememberCopiedText(normalized);
    return;
  }

  throw originalError instanceof Error
    ? originalError
    : new Error('无法写入剪贴板：当前运行环境不支持 Clipboard API 或 fallback copy。');
}

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

export function copyTextToClipboard(text: string): Promise<void> {
  const nav = navigator as NavigatorWithClipboard;
  const originalWriteText =
    typeof nav.clipboard?.writeText === 'function' ? nav.clipboard.writeText.bind(nav.clipboard) : undefined;
  return writeTextWithFallback(String(text), originalWriteText);
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

export function getWorkspaceGit(): Promise<WorkspaceGitSummaryResponse | null> {
  return fetchJson<WorkspaceGitSummaryResponse | null>('/api/workspace/git');
}

export function probeWorkspace(workspace_root: string): Promise<WorkspaceProbeResponse> {
  return fetchJson<WorkspaceProbeResponse>('/api/workspace/probe', undefined, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ workspace_root })
  });
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

export function pickWorkspace(): Promise<WorkspaceSelectResponse | null> {
  return fetchJson<WorkspaceSelectResponse | null>('/api/workspace/pick', undefined, {
    method: 'POST'
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

export function getArtifactGitHistory(
  artifact: string,
  limit = 20
): Promise<ArtifactGitHistoryResponse> {
  return fetchJson<ArtifactGitHistoryResponse>(
    `/api/artifacts/${encodeURIComponent(artifact)}/git/history`,
    { limit }
  );
}

export function getArtifactGitCompare(query: {
  artifact: string;
  base: string;
  head?: string | null;
}): Promise<ArtifactGitCompareResponse> {
  return fetchJson<ArtifactGitCompareResponse>(
    `/api/artifacts/${encodeURIComponent(query.artifact)}/git/compare`,
    {
      base: query.base,
      head: query.head ?? undefined
    }
  );
}

export function rollbackArtifactToRevision(
  artifact: string,
  request: ArtifactGitRollbackRequest
): Promise<ArtifactGitRollbackResponse> {
  return fetchJson<ArtifactGitRollbackResponse>(
    `/api/artifacts/${encodeURIComponent(artifact)}/git/rollback`,
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

export function updateProfileVars(
  profile: string,
  request: ProfileVarsUpdateRequest
): Promise<ProfileDetailResponse> {
  return fetchJson<ProfileDetailResponse>(
    `/api/profiles/${encodeURIComponent(profile)}/vars`,
    undefined,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(request)
    }
  );
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

export function validateMcpServer(serverId: string): Promise<McpValidateResponse> {
  return fetchJson<McpValidateResponse>(`/api/mcp/${encodeURIComponent(serverId)}/validate`, undefined, {
    method: 'POST'
  });
}

export function renderMcpServer(serverId: string): Promise<McpRenderResponse> {
  return fetchJson<McpRenderResponse>(`/api/mcp/${encodeURIComponent(serverId)}/render`);
}

export function testMcpServer(serverId: string): Promise<McpTestResponse> {
  return fetchJson<McpTestResponse>(`/api/mcp/${encodeURIComponent(serverId)}/test`, undefined, {
    method: 'POST'
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
