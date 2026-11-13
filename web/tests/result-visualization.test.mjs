import test from 'node:test';
import assert from 'node:assert/strict';
import {visualizeResult} from '../lib/result-visualization.ts';
test('a state result uses its actual position components',()=>{
 const visual=visualizeResult('state_vector',{columns:['target','observer','epoch_et','x_km','y_km','z_km','range_km'],rows:[[301,399,0,-291608.3847334454,-266716.83330418577,-76102.48704883647,402448.6398904589]]});
 assert.equal(visual.kind,'vector');assert.deepEqual(visual.vector,[-291608.3847334454,-266716.83330418577,-76102.48704883647]);assert.equal(visual.value,402448.6398904589);
});
test('angle display consumes the actual calculated angle',()=>{
 const visual=visualizeResult('phase_angle',{columns:['target','observer','phase_deg'],rows:[[301,399,42.125]]});assert.equal(visual.kind,'angle');assert.equal(visual.degrees,42.125);
});
test('finder results are represented as exact returned event windows',()=>{
 const visual=visualizeResult('gf_distance_search',{columns:['start_et','end_et','duration'],rows:[[651217.8241794102,2370468.738158972,1719250.9139795618]]});assert.equal(visual.kind,'events');assert.deepEqual(visual.intervals,[[651217.8241794102,2370468.738158972]]);
});
test('empty event windows are explicitly represented',()=>{
 const visual=visualizeResult('gf_distance_search',{columns:['start_et','end_et','duration'],rows:[]});assert.equal(visual.kind,'events');assert.deepEqual(visual.intervals,[]);
});
test('moving the timeline inside a finder window preserves its result',async()=>{
 const {resultIsStale}=await import('../lib/result-visualization.ts');
 const data={kind:'finder',resultEpoch:100,searchCenter:100,timelineEpoch:110,resultInputs:'x',currentInputs:'x',resultSpan:20,span:20};
 assert.equal(resultIsStale(data),false);assert.equal(resultIsStale({...data,searchCenter:120}),true);
});
test('malformed finder output is not reported as an empty window',()=>{
 const visual=visualizeResult('gf_distance_search',{columns:['unexpected'],rows:[[4]]});assert.equal(visual.kind,'summary');
});
