import { NextResponse } from 'next/server';
import { invoke, lskPath, readJson, timeBinary } from '../../../lib/server';
export const runtime = 'nodejs';
export async function POST(request: Request) {
  try {
    const body = await readJson(request);
    if (!body || typeof body !== 'object' || Array.isArray(body)) throw new Error('Expected a JSON object.');
    const response = await invoke(timeBinary, [lskPath], JSON.stringify(body));
    return NextResponse.json(JSON.parse(response));
  } catch (error) { return NextResponse.json({error:error instanceof Error ? error.message : 'Time conversion failed'}, {status:400}); }
}
