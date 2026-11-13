type Cell = string | number | boolean | null;
type Result = {columns:string[];rows:Cell[][]};
export type Visualization =
 | {kind:'vector';vector:[number,number,number];value:number|null;description:string}
 | {kind:'angle';degrees:number;description:string}
 | {kind:'events';intervals:[number,number][]}
 | {kind:'summary';fields:{name:string;value:Cell}[]};

export function visualizeResult(operation:string,result:Result):Visualization {
 const {columns,rows}=result;const row=rows[0]||[];
 const get=(name:string):Cell|undefined=>{const i=columns.indexOf(name);return i<0?undefined:row[i];};
 if(operation.startsWith('gf_')){
  const start=columns.indexOf('start_et'),end=columns.indexOf('end_et');
  const intervals:[number,number][]=[];
  if(start<0||end<0)return {kind:'summary',fields:columns.flatMap((name,i)=>row[i]!==undefined?[{name,value:row[i]}]:[]).slice(0,6)};
  for(const r of rows){const a=r[start],b=r[end];if(typeof a!=='number'||!Number.isFinite(a)||typeof b!=='number'||!Number.isFinite(b)||a>b)return {kind:'summary',fields:columns.flatMap((name,i)=>row[i]!==undefined?[{name,value:row[i]}]:[]).slice(0,6)};intervals.push([a,b]);}
  return {kind:'events',intervals};
 }
 const vectorFields:Record<string,{fields:string[];description:string;value?:string}>={
  state_vector:{fields:['x_km','y_km','z_km'],description:'Observer to target position',value:'range_km'},
  sub_solar_point:{fields:['x_km','y_km','z_km'],description:'Target center to sub-solar point'},
  sub_observer_point:{fields:['x_km','y_km','z_km'],description:'Target center to sub-observer point'},
  surface_intercept_point:{fields:['x_km','y_km','z_km'],description:'Target center to surface intercept'},
  tangent_point:{fields:['tangent_x_km','tangent_y_km','tangent_z_km'],description:'Target center to tangent point'},
 };
 const spec=vectorFields[operation];
 if(spec){const values=spec.fields.map(get);if(values.every(v=>typeof v==='number'&&Number.isFinite(v))&&values.some(v=>v!==0)){const value=spec.value?get(spec.value):null;return {kind:'vector',vector:values as [number,number,number],value:typeof value==='number'&&Number.isFinite(value)?value:null,description:spec.description};}}
 const angles:Record<string,{field:string;description:string}>={angular_separation:{field:'separation_deg',description:'Angular separation'},phase_angle:{field:'phase_deg',description:'Phase angle'},angular_size:{field:'angular_size_deg',description:'Angular size'},planetocentric_solar_longitude:{field:'longitude_deg',description:'Planetocentric solar longitude'}};
 const angle=angles[operation];if(angle){const value=get(angle.field);if(typeof value==='number'&&Number.isFinite(value)&&value>=0&&value<=360)return {kind:'angle',degrees:value,description:angle.description};}
 const metadata=new Set(['target','observer','target1','target2','center','illuminator','epoch_et','correction','kind','longitude_deg','latitude_deg','altitude_km','direction_type']);
 return {kind:'summary',fields:columns.flatMap((name,i)=>!metadata.has(name)&&row[i]!==undefined?[{name,value:row[i]}]:[]).slice(0,6)};
}

export function resultIsStale(data:{kind:'geometry'|'finder'|'time';resultEpoch:number|null;searchCenter:number;timelineEpoch:number;resultInputs:string;currentInputs:string;resultSpan:number;span:number}):boolean {
 if(data.resultEpoch===null)return false;
 if(data.resultInputs!==data.currentInputs)return true;
 if(data.kind==='finder')return data.resultEpoch!==data.searchCenter||data.resultSpan!==data.span;
 return data.kind==='geometry'&&data.resultEpoch!==data.timelineEpoch;
}
