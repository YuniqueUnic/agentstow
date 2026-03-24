export type {
  ArtifactDetailResponse,
  ArtifactGitCompareResponse,
  ArtifactGitHistoryResponse,
  ArtifactGitRollbackRequest,
  ArtifactGitRollbackResponse,
  ArtifactKindResponse,
  ArtifactSourceResponse,
  ArtifactSummaryResponse,
  ArtifactSourceUpdateRequest,
  ApiError,
  EnvUsageOwnerKindResponse,
  EnvUsageRefResponse,
  EnvEmitSetSummaryResponse,
  EnvVarSummaryResponse,
  EnvEmitRequest,
  EnvEmitResponse,
  HealthResponse,
  ImpactAnalysisResponse,
  ImpactSubjectKindResponse,
  InstallMethodResponse,
  LinkRecordResponse,
  LinkApplyRequest,
  LinkDesiredInstallResponse,
  LinkOperationActionResponse,
  LinkOperationItemResponse,
  LinkOperationResponse,
  LinkPlanItemResponse,
  LinkPlanRequest,
  LinkRepairRequest,
  LinkStatusResponseItem,
  ManifestResponse,
  ManifestSourceResponse,
  ManifestSourceUpdateRequest,
  McpCheckResponse,
  McpCheckStatusResponse,
  McpHeaderResponse,
  McpRenderResponse,
  McpServerSummaryResponse,
  McpTestResponse,
  McpTransportKindResponse,
  McpValidateResponse,
  ProfileDetailResponse,
  ProfileVarSyntaxModeResponse,
  ProfileVarUpdateItemRequest,
  ProfileSummaryResponse,
  ProfileVarResponse,
  ProfileVarsUpdateRequest,
  RenderResponse,
  ScriptRunRequest,
  ScriptRunResponse,
  ScriptSummaryResponse,
  SecretBindingKindResponse,
  ShellKindResponse,
  TargetSummaryResponse,
  ValidateAsResponse,
  ValidationIssueResponse,
  WatchModeResponse,
  WatchStatusResponse,
  WatchTraceEventResponse,
  WatchTraceLevelResponse,
  WorkspaceCountsResponse,
  WorkspaceGitSummaryResponse,
  WorkspaceInitRequest,
  WorkspaceInitResponse,
  WorkspaceProbeRequest,
  WorkspaceProbeResponse,
  WorkspaceSelectRequest,
  WorkspaceSelectResponse,
  WorkspaceStateResponse,
  WorkspaceSummaryResponse
} from '$lib/bindings';

export type EditorDocumentLanguage =
  | 'auto'
  | 'plaintext'
  | 'jinja'
  | 'toml'
  | 'json'
  | 'html'
  | 'javascript'
  | 'css'
  | 'shell';

export type WorkspacePickerCapability = {
  supported: boolean;
  secureContext: boolean;
  supportsPathExtraction: boolean;
  reason: string | null;
};
