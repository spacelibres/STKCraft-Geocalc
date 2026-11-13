import test from 'node:test';
import assert from 'node:assert/strict';
import {commitSceneSnapshot} from '../lib/scene-snapshot.ts';
const sample=(epoch)=>({epoch_et:epoch,sun:[1,2,3],moon:[4,5,6],systems:[],unavailable:[]});
const trajectories=(epoch)=>({epoch_et:epoch,tracks:[]});
test('a new frame replaces positions and tracks together',()=>{
 const old={sequence:1,scene:sample(0),trajectories:trajectories(0)};
 const next=commitSceneSnapshot(old,2,sample(60),trajectories(60));
 assert.equal(next.scene.epoch_et,60);assert.equal(next.trajectories.epoch_et,60);
});
test('an incomplete or out-of-order update keeps the last matched frame',()=>{
 const old={sequence:2,scene:sample(60),trajectories:trajectories(60)};
 assert.equal(commitSceneSnapshot(old,3,sample(120),null),old);
 assert.equal(commitSceneSnapshot(old,3,sample(120),trajectories(121)),old);
 assert.equal(commitSceneSnapshot(old,1,sample(0),trajectories(0)),old);
});
test('initial positions may render without trajectories when those are unavailable',()=>{
 const initial={sequence:0,scene:null,trajectories:null};
 const next=commitSceneSnapshot(initial,1,sample(0),null);
 assert.equal(next.scene.epoch_et,0);assert.equal(next.trajectories,null);
});
