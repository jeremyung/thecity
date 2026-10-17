import type { BusinessRecipe } from './business-recipes';
/** Landmark-scale office grammars; all dimensions remain authoritative engine inputs. */
export const CORPORATE_SKYSCRAPERS: readonly BusinessRecipe[] = (
  ['split', 'rounded', 'chamfered', 'setback', 'crown'] as const
).map((towerForm, i) => ({
  id: `corporate-${towerForm}`,
  towerForm,
  family: 'office',
  era: 'contemporary',
  label: [
    'Split-volume glass skyscraper',
    'Rounded corporate skyscraper',
    'Chamfered blue skyscraper',
    'Setback corporate skyscraper',
    'Sculpted-crown skyscraper',
  ][i],
  form: 'curtain',
  wall: ['#626762', '#d9dfdf', '#626762', '#a3947e', '#626762'][i],
  roughness: 0.55,
  bay: 1.8,
  upper: 'office',
  retailFloors: 3,
  awning: false,
}));
