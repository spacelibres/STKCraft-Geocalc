import test from 'node:test';
import assert from 'node:assert/strict';
const base=process.env.STKCRAFT_WEB_TEST_URL||'http://127.0.0.1:3100';
test('scene positions use the same geometric state vectors and ET',async()=>{
 const res=await fetch(base+'/api/scene',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({epoch_et:0})});assert.equal(res.status,200);
 const scene=await res.json();assert.equal(scene.epoch_et,0);assert.equal(scene.frame,'J2000');
 assert.deepEqual(scene.moon,[-291608.3847334454,-266716.83330418577,-76102.48704883647]);
 assert.ok(Math.hypot(...scene.sun)>140000000);assert.ok(Math.hypot(...scene.moon)<500000);
});
test('solar-system positions are computed in J2000 and explicitly label barycenters',async()=>{
 const res=await fetch(base+'/api/scene',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({epoch_et:0})});assert.equal(res.status,200);const scene=await res.json();
 assert.equal(scene.systems.length,8);assert.deepEqual(scene.systems.map(body=>body.id),[199,299,4,5,6,7,8,9]);
 assert.equal(scene.systems.find(body=>body.id===5).kind,'barycenter');
 for(const body of scene.systems)assert.equal(body.position.length,3);
});
test('trajectory samples retain actual SPICE positions at the selected epoch',async()=>{
 const res=await fetch(base+'/api/trajectories',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({epoch_et:0})});assert.equal(res.status,200);const data=await res.json();
 const sceneResponse=await fetch(base+'/api/scene',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({epoch_et:0})});const scene=await sceneResponse.json();
 assert.equal(data.tracks.length,9);assert.equal(data.tracks[0].samples.length,65);
 assert.deepEqual(data.tracks.find(track=>track.id===199).samples[32],scene.systems.find(body=>body.id===199).position);
});
