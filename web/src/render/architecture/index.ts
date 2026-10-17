import {applyBuildingWeathering,buildingWeathering} from './character';
import type {Building} from '../../types';
import type {ArtMaterials} from '../materials';
import {apartmentPilot} from './apartment';
import {apartmentRecipe,supportsApartment,type ApartmentBuildOptions} from './recipes';
export {APARTMENT_RECIPES,APARTMENT_RECIPE_IDS,APARTMENT_RECIPE_VERSION,apartmentRecipe,supportsApartment} from './recipes';
export type {ApartmentBuildOptions,ApartmentRecipeId} from './recipes';
export {disposeApartmentComponentTemplates} from './components';
/** Integration boundary: local-space geometry only. Caller owns placement, rotation and disposal.
 * null means use the existing family renderer; no DOM, timers, network, or engine mutations.
 */
export function buildApartmentBuilding(building:Building,materials:ArtMaterials,options:ApartmentBuildOptions={}){
 if(!supportsApartment(building))return null;
 const group=apartmentPilot(building,materials,options);
 const weather=buildingWeathering(building,apartmentRecipe(building,options.recipe));
 applyBuildingWeathering(group,weather);group.userData.apartmentPilot.weathering=weather;
 return group;
}

export {buildResidentialTower,supportsResidentialTower,RESIDENTIAL_TOWERS} from './residential-tower';
export type {ResidentialTowerRecipe,ResidentialTowerOptions} from './residential-tower';
