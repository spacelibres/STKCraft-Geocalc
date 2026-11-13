import { NextResponse } from 'next/server';
import { kernelConfig, mcp, nasaComparison, readJson, tools } from '../../../lib/server';
export const runtime = 'nodejs';
export async function POST(request: Request) {
  try {
    const body = await readJson(request);
    if (!body || typeof body !== 'object' || Array.isArray(body)) throw new Error('Expected a JSON object.');
    const { operation, parameters } = body as {operation?: unknown; parameters?: unknown};
    if (typeof operation !== 'string' || !parameters || typeof parameters !== 'object' || Array.isArray(parameters) || 'kernels' in parameters) throw new Error('Provide an operation and parameters; kernels are configured by the server.');
    const catalog = await tools();
    if (!catalog.some(tool => tool.name === operation)) throw new Error('Unknown calculation operation.');
    const result = await mcp('tools/call', {name:operation, arguments:{...parameters, kernels:kernelConfig()}});
    const comparison = await nasaComparison(operation, parameters, (result as {structuredContent?: unknown}).structuredContent);
    return NextResponse.json({...result as object, nasaComparison:comparison});
  } catch (error) { return NextResponse.json({error:error instanceof Error ? error.message : 'Calculation failed'}, {status:400}); }
}
