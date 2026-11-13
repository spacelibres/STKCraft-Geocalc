import { NextResponse } from 'next/server';
import { tools, timeTools } from '../../../lib/server';
export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';
export async function GET() {
  try { const [geometry,time] = await Promise.all([tools(),timeTools().then(value=>({tools:value,error:null})).catch(error=>({tools:[],error:error instanceof Error?error.message:'Time backend unavailable'}))]); return NextResponse.json({ tools:geometry, timeTools:time.tools, timeError:time.error }); }
  catch (error) { return NextResponse.json({error: error instanceof Error ? error.message : 'Backend unavailable'}, {status:503}); }
}
