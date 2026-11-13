import {NextResponse} from 'next/server';
import {kernelConfig,mcp,readJson} from '../../../lib/server';
export const runtime='nodejs';
type Structured={structuredContent?:{columns:string[];rows:unknown[][]}};
export async function POST(request:Request){
 try{
  const input=await readJson(request) as {epoch_et?:unknown};
  if(!input||typeof input.epoch_et!=='number'||!Number.isFinite(input.epoch_et))throw new Error('A finite ET is required.');
  const kernels=kernelConfig();
  const position=async(target:number,observer=399):Promise<number[]>=>{
   const response=await mcp('tools/call',{name:'state_vector',arguments:{target,observer,epoch_et:input.epoch_et,correction:'NONE',kernels}}) as Structured;
   const data=response.structuredContent;
   if(!data||data.rows.length!==1)throw new Error('No scene position was returned.');
   const v=['x_km','y_km','z_km'].map(name=>data.rows[0][data.columns.indexOf(name)]);
   if(!v.every(value=>typeof value==='number'&&Number.isFinite(value)))throw new Error('Invalid scene position.');
   return v as number[];
  };
  const definitions=[{id:199,name:'Mercury',kind:'center'},{id:299,name:'Venus',kind:'center'},{id:4,name:'Mars system',kind:'barycenter'},{id:5,name:'Jupiter system',kind:'barycenter'},{id:6,name:'Saturn system',kind:'barycenter'},{id:7,name:'Uranus system',kind:'barycenter'},{id:8,name:'Neptune system',kind:'barycenter'},{id:9,name:'Pluto system',kind:'barycenter'}];
  const [sun,moon,systemResults]=await Promise.all([position(10),position(301),Promise.allSettled(definitions.map(body=>position(body.id,10)))]);
  const systems=definitions.flatMap((body,index)=>{const result=systemResults[index];return result.status==='fulfilled'?[{...body,position:result.value}]:[];});
  const unavailable=definitions.flatMap((body,index)=>systemResults[index].status==='rejected'?[body.name]:[]);
  return NextResponse.json({epoch_et:input.epoch_et,frame:'J2000',observer:'Earth center',correction:'NONE',sun,moon,systems,unavailable});
 }catch(error){return NextResponse.json({error:error instanceof Error?error.message:'Scene calculation failed.'},{status:400});}
}
