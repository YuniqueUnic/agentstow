const latestFailedAutoPreviewFingerprintByDocument = new Map<string, string>();

export function getLatestFailedAutoPreviewFingerprint(documentScope: string): string | undefined {
  return latestFailedAutoPreviewFingerprintByDocument.get(documentScope);
}

export function rememberFailedAutoPreviewFingerprint(
  documentScope: string,
  fingerprint: string
): void {
  latestFailedAutoPreviewFingerprintByDocument.set(documentScope, fingerprint);
}

export function clearFailedAutoPreviewFingerprint(documentScope: string): void {
  latestFailedAutoPreviewFingerprintByDocument.delete(documentScope);
}
