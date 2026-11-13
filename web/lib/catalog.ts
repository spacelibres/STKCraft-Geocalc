export type FieldSchema = {type?:string; default?:unknown; description?:string; enum?:string[]};
export type Tool = {name:string;description:string;inputSchema:{properties:Record<string,FieldSchema>;required?:string[]}};
export type Calculation = {columns:string[];rows:(string|number|boolean|null)[][];rawCsv:string;backend:string};
export const names:Record<string,[string,string]> = {
 state_vector:['State vector','Position and velocity relative to an observer'], angular_separation:['Angular separation','Angle between two targets'], angular_size:['Angular size','Apparent size of a target'], frame_transformation:['Frame transformation','Orientation between reference frames'], illumination_angles:['Illumination angles','Surface phase, incidence and emission'], phase_angle:['Phase angle','Illuminator, target and observer'], pointing_direction:['Pointing direction','A vector in a reference frame'], sub_solar_point:['Sub-solar point','Illuminator point on a target'], sub_observer_point:['Sub-observer point','Observer point on a target'], surface_intercept_point:['Surface intercept','Ray intersection with a target'], tangent_point:['Tangent point','Closest point of a ray to a target'], osculating_elements:['Orbital elements','Instantaneous orbit parameters'], planetocentric_solar_longitude:['Longitude of the Sun','Planetocentric solar longitude'],
 gf_coordinate_search:['Position finder','Search a position coordinate'], gf_angular_separation_search:['Angular separation finder','Search an angle between targets'], gf_distance_search:['Distance finder','Search observer–target distance'], gf_sub_point_search:['Sub-point finder','Search a sub-observer coordinate'], gf_occultation_search:['Occultation finder','Search occultation intervals'], gf_surface_intercept_point_search:['Surface intercept finder','Search an intercept coordinate'], gf_target_in_instrument_fov_search:['Target in field of view','Search instrument visibility'], gf_ray_in_fov_search:['Ray in field of view','Search ray visibility'], gf_range_rate_search:['Range rate finder','Search a range-rate condition'], gf_phase_angle_search:['Phase angle finder','Search a phase-angle condition'], gf_illumination_angles_search:['Illumination finder','Search an illumination condition'],
};
export function label(name:string):string {return names[name]?.[0] || name.replaceAll('_',' ').replace(/\b\w/g, c=>c.toUpperCase()).replace(/\b(Utc|Tai|Tt|Et|Gps|Ut1|Sclk|Lsk|Wgc|Tow)\b/g, value=>value.toUpperCase());}
export function defaultValue(name:string, field:FieldSchema):string {
 if(name==='year')return '2024'; if(name==='month'||name==='day')return '1';
 if(name==='hour'||name==='minute'||name==='second')return '0';
 if (name==='target'||name==='target1') return '301'; if (name==='observer') return '399';
 if (name==='target2') return '10'; if (name==='step_seconds') return '3600';
 if (name==='frame1') return 'J2000'; if(name==='frame2') return 'IAU_MOON';
 if (name==='epoch_et'||name==='confinement'||name==='kernels') return '';
 if (field.default!==undefined && field.default!==null && !String(field.default).startsWith('@')) return String(field.default);
 return '';
}
export function parseUtc(text:string) {
 const match=/^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2}):(\d{2}(?:\.\d+)?)(?:Z)?$/.exec(text.trim());
 if(!match) throw new Error('Use UTC in YYYY-MM-DDTHH:MM:SS format.');
 const [year,month,day,hour,minute,second]=match.slice(1).map(Number);
 return {year,month,day,hour,minute,second};
}
export function formatUtc(value:{year:number;month:number;day:number;hour:number;minute:number;second:number}) {
 const pad=(n:number)=>String(n).padStart(2,'0');
 const second=String(value.second).padStart(2,'0');
 return `${value.year}-${pad(value.month)}-${pad(value.day)}T${pad(value.hour)}:${pad(value.minute)}:${second}Z`;
}
export async function requestJson(url:string,body?:unknown) {
 const response=await fetch(url,body===undefined ? {cache:'no-store'} : {method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});
 const data=await response.json();
 if(!response.ok) throw new Error(data.error || `Request failed (${response.status})`);
 return data;
}
