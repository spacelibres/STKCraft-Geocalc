import { spawn } from 'node:child_process';
import path from 'node:path';
import { access } from 'node:fs/promises';

export type ToolSchema = { name: string; description: string; inputSchema: { properties: Record<string, { type?: string; default?: unknown; description?: string }>; required?: string[] } };
export type KernelConfig = { spk: string[]; pck?: string[]; fk?: string[]; lsk?: string; ck?: string[]; ik?: string; instrument_id?: number; sclk?: { spacecraft_id: number; path: string }[] };
const root = path.resolve(process.cwd(), '..');
const executable = (name: string) => process.platform === 'win32' ? `${name}.exe` : name;
export const geometryBinary = process.env.STKCRAFT_WEB_GEOMETRY ? path.resolve(root,process.env.STKCRAFT_WEB_GEOMETRY) : path.join(root,'target','release',executable('wgc_geometry'));
export const mcpBinary = process.env.STKCRAFT_WEB_MCP ? path.resolve(root,process.env.STKCRAFT_WEB_MCP) : path.join(root, 'target', 'release', executable('stkcraft-geocalc-mcp'));
export const timeBinary = process.env.STKCRAFT_WEB_TIME ? path.resolve(root,process.env.STKCRAFT_WEB_TIME) : path.join(process.cwd(), 'native', 'target', 'release', executable('stkcraft-web-time-bridge'));
export const timeMcpBinary = process.env.STKCRAFT_WEB_TIME_MCP ? path.resolve(root,process.env.STKCRAFT_WEB_TIME_MCP) : path.join(process.cwd(), 'native', 'target', 'release', executable('stkcraft-web-time-mcp'));
export const lskPath = process.env.STKCRAFT_WEB_LSK ? path.resolve(root,process.env.STKCRAFT_WEB_LSK) : path.join(root, 'target', 'kernels', 'naif0012.tls');
const kernel = (name: string) => path.join(root, 'target', 'kernels', name);
export async function readJson(request: Request): Promise<unknown> {
  const declared = request.headers.get('content-length');
  if (declared && (!/^\d+$/.test(declared) || Number(declared) > 65536)) throw new Error('Request body exceeds the 64 KiB limit.');
  if (!request.body) throw new Error('Request body is required.');
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const {done, value} = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > 65536) { await reader.cancel(); throw new Error('Request body exceeds the 64 KiB limit.'); }
    chunks.push(value);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}
export function kernelConfig(): KernelConfig {
  if (process.env.STKCRAFT_WEB_KERNELS_JSON) {
    const value: unknown = JSON.parse(process.env.STKCRAFT_WEB_KERNELS_JSON);
    if (!value || typeof value !== 'object' || !Array.isArray((value as KernelConfig).spk) || !(value as KernelConfig).spk.length) throw new Error('Configured kernel context must include an ordered SPK list.');
    const configured = value as KernelConfig;
    if (configured.lsk !== undefined && (typeof configured.lsk !== 'string' || path.resolve(root,configured.lsk) !== lskPath)) throw new Error('The geometry LSK must match STKCRAFT_WEB_LSK.');
    return {...configured, lsk:lskPath};
  }
  return { spk: [kernel('de442.bsp')], pck: [kernel('pck00011.tpc'), kernel('mars_iau2000_v1.tpc'), kernel('gm_de431.tpc'), kernel('moon_pa_de440_200625.bpc')], fk: [kernel('moon_de440_250416.tf'), kernel('moon_assoc_me.tf')], lsk: lskPath };
}
export async function invoke(binary: string, args: string[], input: string, overrides: Record<string,string> = {}): Promise<string> {
  if (path.isAbsolute(binary) || binary.includes('/') || binary.includes('\\')) await access(binary);
  return new Promise((resolve, reject) => {
    const child = spawn(binary, args, { cwd: root, windowsHide: true, shell: false, env: {...process.env,...overrides} });
    let stdout = '', stderr = '', settled = false;
    const finish = (error?: Error) => { if (settled) return; settled = true; clearTimeout(timer); error ? reject(error) : resolve(stdout); };
    const timer = setTimeout(() => { child.kill(); finish(new Error('The computation exceeded the server timeout.')); }, 120_000);
    child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk: string) => { stdout += chunk; if (stdout.length > 3_000_000) { child.kill(); finish(new Error('Backend output is too large.')); } });
    child.stderr.on('data', (chunk: string) => { stderr = (stderr + chunk).slice(-8192); });
    child.on('error', error => finish(error));
    child.on('close', code => code === 0 ? finish() : finish(new Error(stderr || stdout || `Backend exited ${code}`)));
    child.stdin.on('error', error => finish(error));
    child.stdin.end(input);
  });
}
export async function mcp(method: string, params: Record<string, unknown> = {}, binary = mcpBinary): Promise<unknown> {
  const messages = [
    {jsonrpc:'2.0', id:1, method:'initialize', params:{protocolVersion:'2025-11-25', capabilities:{}, clientInfo:{name:'stkcraft-webgeocalc',version:'0.1.0'}}},
    {jsonrpc:'2.0', method:'notifications/initialized'},
    {jsonrpc:'2.0', id:2, method, params},
  ];
  const output = await invoke(binary, [], messages.map(x => JSON.stringify(x)).join('\n') + '\n');
  const lines = output.trim().split('\n').filter(Boolean).map(line => JSON.parse(line));
  const response = lines.find(item => item.id === 2);
  if (!response) throw new Error('The geometry backend returned no response.');
  if (response.error) throw new Error(response.error.message || 'Geometry request rejected.');
  if (response.result?.isError) throw new Error(response.result.content?.[0]?.text || 'Geometry calculation failed.');
  return response.result;
}
export async function tools(): Promise<ToolSchema[]> {
  const result = await mcp('tools/list') as {tools: ToolSchema[]};
  if (!Array.isArray(result.tools)) throw new Error('Invalid geometry catalog.');
  return result.tools.filter(tool => tool.name !== 'calculate_csv');
}
export async function timeTools(): Promise<ToolSchema[]> {
  const result = await mcp('tools/list', {}, timeMcpBinary) as {tools: ToolSchema[]};
  if (!Array.isArray(result.tools)) throw new Error('Invalid time catalog.');
  return result.tools;
}

export async function nasaComparison(operation: string, parameters: unknown, local: unknown): Promise<unknown> {
  try {
    const script = path.join(root, 'web', 'nasa', 'compare.py');
    const output = await invoke(process.env.STKCRAFT_WEB_PYTHON || 'python', [script], JSON.stringify({operation, parameters, local}));
    return JSON.parse(output);
  } catch (error) { return {status:'unavailable', message:error instanceof Error ? error.message : 'NASA comparison unavailable.'}; }
}
