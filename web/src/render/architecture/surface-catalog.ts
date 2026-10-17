import { APARTMENT_RECIPES } from './recipes';
import { RESIDENTIAL_TOWERS } from './tower-recipes';
export const ARCHITECTURE_TEXTURES = [
  'warm-plaster',
  'cast-concrete',
  'old-mineral-plaster',
  'old-sand-coat',
  'old-concrete',
  'old-brown-ceramic',
  'old-taupe-ceramic',
  'new-ivory-porcelain',
  'new-honed-limestone',
  'new-white-precast',
  'new-bronze-metal',
  'new-charcoal-ceramic',
] as const;
export type ArchitectureTexture = (typeof ARCHITECTURE_TEXTURES)[number];
const recipeTexture: Record<keyof typeof APARTMENT_RECIPES, ArchitectureTexture> = {
  gallery: 'old-mineral-plaster',
  walkup: 'old-sand-coat',
  terrace: 'old-concrete',
  compact: 'old-taupe-ceramic',
  brick: 'old-brown-ceramic',
  modern: 'new-white-precast',
  urban: 'new-bronze-metal',
  ceramic: 'new-ivory-porcelain',
  limestone: 'new-honed-limestone',
  courtyard: 'new-charcoal-ceramic',
};
export interface ArchitectureSurface {
  hex: string;
  roughness: number;
  role: string;
  texture?: ArchitectureTexture;
  finish?: 'brick' | 'tile';
}
/** Bounded shared material palette. Never add a material or image for an individual seed. */
export const ARCHITECTURE_SURFACES: ArchitectureSurface[] = [
  ...Object.values(APARTMENT_RECIPES).flatMap((r) =>
    r.wallTints.map((hex) => ({
      hex,
      roughness: r.wallRoughness ?? 0.85,
      role: r.id === 'urban' ? 'metal' : (r.finish ?? 'plaster'),
      texture: recipeTexture[r.id],
      finish: r.finish,
    })),
  ),
  ...[...new Set(Object.values(APARTMENT_RECIPES).map((r) => r.concreteTint))].map((hex) => ({
    hex,
    roughness: 0.85,
    role: 'concrete',
    texture: 'cast-concrete' as const,
  })),
  ...Object.values(RESIDENTIAL_TOWERS).flatMap((r) => [
    { hex: r.wall, roughness: 0.85, role: 'cladding', texture: r.texture },
    {
      hex: r.slab,
      roughness: 0.85,
      role: 'slab',
      texture: r.form === 'ribbon' ? ('new-bronze-metal' as const) : ('new-white-precast' as const),
    },
    { hex: r.metal, roughness: 0.45, role: 'metal', texture: 'new-bronze-metal' as const },
    { hex: r.stone, roughness: 0.85, role: 'stone', texture: 'new-honed-limestone' as const },
  ]),
  ...['#59717a', '#40565f', '#7c8c8b', '#526c73'].map((hex) => ({
    hex,
    roughness: 0.24,
    role: 'glass',
  })),
];
