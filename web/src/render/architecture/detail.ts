import type {ApartmentBuildOptions} from './recipes';
export type ApartmentDetail=NonNullable<ApartmentBuildOptions['detail']>;
/** Camera-to-building-center distance; wide hysteresis prevents rebuilds at threshold jitter. */
export function apartmentDetail(distance:number,previous?:ApartmentDetail):ApartmentDetail{
 if(previous==='close'&&distance<130)return 'close';
 if(previous==='far'&&distance>300)return 'far';
 if(distance<95)return 'close';
 return distance>360?'far':'district';
}
