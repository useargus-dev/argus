/** Matches `generate_proxy_placeholder` in `src-tauri/src/db/bucket_mappings.rs`. */
const TOKEN_CHARS =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const TOKEN_LEN = 24;
const MAX_ATTEMPTS = 64;

/** Client-side preview only — server enforces uniqueness on persist. */
export function generateProxyToken(usedTokens: ReadonlySet<string> = new Set()): string {
  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    let token = "";
    const random = crypto.getRandomValues(new Uint8Array(TOKEN_LEN));
    for (let i = 0; i < TOKEN_LEN; i++) {
      token += TOKEN_CHARS[random[i]! % TOKEN_CHARS.length];
    }
    const candidate = `argus-proxy-${token}`;
    if (!usedTokens.has(candidate)) {
      return candidate;
    }
  }
  throw new Error("Could not generate a unique proxy token preview");
}

export function collectBucketProxyTokens(
  mappings: { proxyPlaceholder: string | null; id?: string }[],
  excludeMappingId?: string | null,
): Set<string> {
  const used = new Set<string>();
  for (const m of mappings) {
    if (excludeMappingId && m.id === excludeMappingId) continue;
    if (m.proxyPlaceholder) used.add(m.proxyPlaceholder);
  }
  return used;
}
