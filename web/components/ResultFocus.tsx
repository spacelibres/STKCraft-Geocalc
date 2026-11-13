'use client';
import type {Visualization} from '../lib/result-visualization';
import {label} from '../lib/catalog';

export default function ResultFocus({visual,operation,epoch,frame,context}:{visual:Visualization;operation:string;epoch:number|null;frame:string;context:string}){
 const radians=visual.kind==='angle'?visual.degrees*Math.PI/180:0;
 const endX=45+30*Math.cos(radians),endY=75-30*Math.sin(radians);
 return <section className="result-focus" aria-label="Calculation visualization" aria-live="polite">
  <div className="focus-kicker">{epoch===null?'TIME CONVERSION':`CALCULATED RESULT · ET ${String(epoch)} s`}</div>
  <h3>{label(operation)}</h3>
  {visual.kind==='vector'&&<><p>{context}</p><p>{visual.description} · {frame}</p><div className="focus-coordinates">{visual.vector.map((value,index)=><div key={index}><small>{['X','Y','Z'][index]} · KM</small><strong>{String(value)}</strong></div>)}</div>{visual.value!==null&&<p className="focus-distance">Distance <strong>{String(visual.value)} km</strong></p>}<p className="focus-caption">3D vector direction uses the computed components. Length is normalized; endpoint markers are illustrative.</p></>}
  {visual.kind==='angle'&&<><div className="focus-angle"><svg viewBox="0 0 120 115" role="img" aria-label={`${visual.description}: ${String(visual.degrees)} degrees`}><line x1="45" y1="75" x2="110" y2="75"/><line x1="45" y1="75" x2={45+65*Math.cos(radians)} y2={75-65*Math.sin(radians)}/>{visual.degrees===360?<circle cx="45" cy="75" r="30"/>:visual.degrees!==0&&<path d={`M 75 75 A 30 30 0 ${visual.degrees>180?1:0} 0 ${endX} ${endY}`}/>}<circle className="focus-origin" cx="45" cy="75" r="3"/></svg><div><small>{visual.description}</small><strong>{String(visual.degrees)}°</strong></div></div><p className="focus-caption">The angle is computed; ray orientation is schematic.</p></>}
  {visual.kind==='events'&&<><p>{visual.intervals.length===0?'No intervals satisfy this condition.':`${visual.intervals.length} ${visual.intervals.length===1?'interval':'intervals'} found.`}</p><div className="focus-intervals">{visual.intervals.slice(0,3).map(([start,end],i)=><div key={i}><small>INTERVAL {i+1} · ET SECONDS</small><strong>{String(start)} → {String(end)}</strong></div>)}</div>{visual.intervals.length>3&&<p>See the result table for all intervals.</p>}<a href="#timeline">View intervals on timeline ↓</a></>}
  {visual.kind==='summary'&&<><p>Computed values</p><div className="focus-coordinates">{visual.fields.map(field=><div key={field.name}><small>{label(field.name)}</small><strong>{String(field.value??'—')}</strong></div>)}</div><p className="focus-caption">A spatial illustration for this calculation is not yet available.</p></>}
 </section>;
}
