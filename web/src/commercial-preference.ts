export const COMMERCIAL_DISTRICTS = [
  ['standard', 'Standard city'],
  ['tokyo-mix', 'Tokyo district mix'],
  ['shibuya', 'Shibuya · fashion & media'],
  ['roppongi', 'Roppongi · dining & nightlife'],
  ['akihabara', 'Akihabara · electronics & games'],
  ['shinjuku', 'Shinjuku · entertainment'],
  ['ameyoko', 'Ameyoko · market street'],
] as const;
export type CommercialDistrict = (typeof COMMERCIAL_DISTRICTS)[number][0];
export function parseCommercialDistrict(value: unknown): CommercialDistrict {
  return COMMERCIAL_DISTRICTS.some(([id]) => id === value)
    ? (value as CommercialDistrict)
    : 'standard';
}
export type DisplayMode = 'off' | 'static' | 'animated';
export function parseDisplayMode(value: unknown): DisplayMode {
  return value === 'off' || value === 'static' ? value : 'animated';
}
