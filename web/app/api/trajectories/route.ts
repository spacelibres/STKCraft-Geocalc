import {NextResponse} from 'next/server';
import {geometryBinary,invoke,kernelConfig,readJson} from '../../../lib/server';
export const runtime='nodejs';
const tracks=[{id:199,years:.3},{id:299,years:.7},{id:399,years:1},{id:4,years:2},{id:5,years:12},{id:6,years:30},{id:7,years:84},{id:8,years:165},{id:9,years:250}];
export async function POST(request:Request){
 try{
  const body=await readJson(request) as {epoch_et?:unknown};
  if(!body||typeof body.epoch_et!=='number'||!Number.isFinite(body.epoch_et))throw new Error('A finite ET is required.');
  const epoch=body.epoch_et,kernels=kernelConfig(),lines:string[]=[];
  for(const track of tracks)for(let i=0;i<=64;i++)lines.push(`${track.id},10,${epoch+(i/64-.5)*track.years*31557600},NONE`);
  if(kernels.spk.some(path=>path.includes(',')))throw new Error('SPK paths containing commas are not supported by the trajectory backend.');
  const output=await invoke(geometryBinary,['state'],lines.join('\n')+'\n',{STKCRAFT_SPK:kernels.spk.join(',')});
  const rows=output.trim().split(/\r?\n/);if(rows.length!==lines.length)throw new Error('Trajectory backend returned an invalid row count.');
  const result=tracks.map((track,index)=>{
   const samples=rows.slice(index*65,(index+1)*65).map(row=>{if(row.startsWith('ERROR,'))return null;const fields=row.split(',');const v=fields.slice(4,7).map(Number);return fields.length===14&&v.length===3&&v.every(Number.isFinite)?v:null;});
   return {...track,samples};
  });
  return NextResponse.json({epoch_et:epoch,frame:'J2000',observer:'Sun center',correction:'NONE',tracks:result});
 }catch(error){return NextResponse.json({error:error instanceof Error?error.message:'Trajectory computation failed.'},{status:400});}
}
