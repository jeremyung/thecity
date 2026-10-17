export const APARTMENT_ERAS = ['mixed', 'older', 'contemporary', 'legacy'] as const;
export type ApartmentEra = (typeof APARTMENT_ERAS)[number];
export function parseApartmentEra(value: unknown): ApartmentEra {
  return APARTMENT_ERAS.includes(value as ApartmentEra) ? (value as ApartmentEra) : 'mixed';
}
