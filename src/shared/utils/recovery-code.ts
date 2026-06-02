const RECOVERY_ALPHABET = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";

export function normalizeRecoveryCode(raw: string): string {
  return raw
    .replace(/[-\s]/g, "")
    .toUpperCase()
    .replace(/[^23456789ABCDEFGHJKMNPQRSTUVWXYZ]/g, "")
    .slice(0, 8);
}

export function formatRecoveryCode(raw: string): string {
  const code = normalizeRecoveryCode(raw);
  if (code.length <= 4) return code;
  return `${code.slice(0, 4)}-${code.slice(4, 8)}`;
}

export function isValidRecoveryCode(raw: string): boolean {
  const code = normalizeRecoveryCode(raw);
  return code.length === 8 && [...code].every((c) => RECOVERY_ALPHABET.includes(c));
}

export function splitRecoveryCode(raw: string): [string, string] {
  const code = normalizeRecoveryCode(raw);
  return [code.slice(0, 4), code.slice(4, 8)];
}

export function joinRecoveryCode(part1: string, part2: string): string {
  return normalizeRecoveryCode(`${part1}${part2}`);
}
