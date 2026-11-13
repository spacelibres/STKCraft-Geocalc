import {NextResponse} from 'next/server';
import {invoke,lskPath,mcp,nasaComparison,readJson,timeBinary,timeMcpBinary,timeTools} from '../../../../lib/server';
export const runtime='nodejs';
export async function POST(request:Request){
 try {
  const body=await readJson(request);
  if(!body||typeof body!=='object'||Array.isArray(body))throw new Error('Expected a JSON object.');
  const {operation,parameters}=body as {operation?:unknown;parameters?:unknown};
  if(typeof operation!=='string'||!parameters||typeof parameters!=='object'||Array.isArray(parameters))throw new Error('Provide an operation and parameters.');
  const args=parameters as Record<string,unknown>;
  const catalog=await timeTools();const tool=catalog.find(item=>item.name===operation);
  if(!tool)throw new Error('Unknown time operation.');
  const properties=tool.inputSchema.properties;
  for(const key of Object.keys(args)){if(['lsk_path','eop_path','sclk_path'].includes(key))throw new Error('Time kernel paths are configured by the server.');if(!(key in properties))throw new Error(`Unknown parameter: ${key}`);}
  if(operation==='utc_to_et'||operation==='et_to_utc'){
   const payload=operation==='utc_to_et'?{direction:operation,utc:args,calendar:args.calendar}:{direction:operation,et:args.et,calendar:args.calendar};
   const value=JSON.parse(await invoke(timeBinary,[lskPath],JSON.stringify(payload)));
   const normalized=operation==='et_to_utc'?{et:args.et,...value}:value;
   const comparison=await nasaComparison(operation,args,{columns:Object.keys(normalized),rows:[Object.values(normalized)]});
   return NextResponse.json({operation,value:normalized,contract:'STKCraft-time TimeContext / NAIF ET',nasaComparison:comparison});
  }
  const callArgs:Record<string,unknown>={...args};
  if('lsk_path' in properties)callArgs.lsk_path=lskPath;
  for(const [key,env] of [['eop_path','STKCRAFT_WEB_EOP'],['sclk_path','STKCRAFT_WEB_SCLK']] as const){
   if(key in properties){const configured=process.env[env];if(!configured && tool.inputSchema.required?.includes(key))throw new Error(`${env} must be configured on the server for this operation.`);if(configured)callArgs[key]=configured;}
  }
  const response=await mcp('tools/call',{name:operation,arguments:callArgs},timeMcpBinary) as {content?:{text?:string}[]};
  const text=response.content?.[0]?.text;if(!text)throw new Error('Time backend returned no value.');
  const value=JSON.parse(text);
  const comparison=await nasaComparison(operation,args,{columns:Object.keys(value),rows:[Object.values(value)]});
  return NextResponse.json({operation,value,contract:'STKCraft-time MCP',nasaComparison:comparison});
 }catch(error){return NextResponse.json({error:error instanceof Error?error.message:'Time conversion failed'},{status:400});}
}
