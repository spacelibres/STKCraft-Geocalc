'use client';
import {useEffect,useRef} from 'react';
import * as THREE from 'three';
import {OrbitControls} from 'three/examples/jsm/controls/OrbitControls.js';

export type SceneData={epoch_et:number;sun:[number,number,number];moon:[number,number,number];systems:{id:number;name:string;kind:string;position:[number,number,number]}[];unavailable:string[]};
export type TrajectoryData={epoch_et:number;tracks:{id:number;years:number;samples:([number,number,number]|null)[]}[]};
type View='solar-system'|'sun-earth'|'earth-moon'|'result-vector';
const toScene=(v:[number,number,number])=>new THREE.Vector3(v[0],v[2],-v[1]);
// Equatorial radii in km, from NAIF pck00011.tpc. Bodies are spheres for the display.
const radii={sun:695700,earth:6378.1366,moon:1737.4};
export default function SpaceScene({data,view,trajectories,resultVector}:{data:SceneData|null;view:View;trajectories:TrajectoryData|null;resultVector:[number,number,number]|null}) {
 const mount=useRef<HTMLDivElement>(null);
 const active=useRef<{scene:THREE.Scene;camera:THREE.PerspectiveCamera;renderer:THREE.WebGLRenderer;controls:OrbitControls;sun:THREE.Mesh;earth:THREE.Mesh;moon:THREE.Mesh;light:THREE.DirectionalLight;markers:THREE.Points;traces:THREE.Group;result:THREE.Group;initialized:boolean;view:View|null}|null>(null);
 useEffect(()=>{
  const host=mount.current;if(!host)return;
  const scene=new THREE.Scene();scene.background=new THREE.Color('#080e1b');
  const camera=new THREE.PerspectiveCamera(36,1,.00001,10000);
  const renderer=new THREE.WebGLRenderer({antialias:true});renderer.setPixelRatio(Math.min(devicePixelRatio,2));renderer.outputColorSpace=THREE.SRGBColorSpace;host.appendChild(renderer.domElement);
  const controls=new OrbitControls(camera,renderer.domElement);controls.enableDamping=true;controls.minDistance=.0001;controls.maxDistance=200;
  scene.add(new THREE.AmbientLight('#a4c5e5',1.1));const light=new THREE.DirectionalLight('#fff4dc',2.5);scene.add(light);
  const make=(color:string,emissive:string)=>{const body=new THREE.Mesh(new THREE.SphereGeometry(1,32,24),new THREE.MeshPhongMaterial({color,emissive,shininess:20}));body.visible=false;scene.add(body);return body;};
  const sun=make('#ffd184','#c8781f'),earth=make('#337e9c','#061e30'),moon=make('#b8c4c8','#141c22');
  const spriteCanvas=document.createElement('canvas');spriteCanvas.width=64;spriteCanvas.height=64;
  const spriteContext=spriteCanvas.getContext('2d');
  if(!spriteContext)throw new Error('Unable to create circular point markers.');
  spriteContext.fillStyle='#fff';spriteContext.beginPath();spriteContext.arc(32,32,30,0,Math.PI*2);spriteContext.fill();
  const pointTexture=new THREE.CanvasTexture(spriteCanvas);
  const coords=new Float32Array(750*3);let seed=128773;for(let i=0;i<coords.length;i+=3){seed=(seed*1664525+1013904223)>>>0;coords[i]=(seed/4294967296-.5)*500;seed=(seed*1664525+1013904223)>>>0;coords[i+1]=(seed/4294967296-.5)*500;seed=(seed*1664525+1013904223)>>>0;coords[i+2]=(seed/4294967296-.5)*500;}
  const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.BufferAttribute(coords,3));scene.add(new THREE.Points(geometry,new THREE.PointsMaterial({color:'#b5d4dd',size:.08,map:pointTexture,transparent:true,alphaTest:.05,depthWrite:false,opacity:.55})));
  const markers=new THREE.Points(new THREE.BufferGeometry(),new THREE.PointsMaterial({vertexColors:true,size:8,sizeAttenuation:false,map:pointTexture,transparent:true,alphaTest:.05,depthWrite:false}));scene.add(markers);
  const traces=new THREE.Group();scene.add(traces);const result=new THREE.Group();scene.add(result);
  const resize=()=>{const w=host.clientWidth,h=host.clientHeight;if(!w||!h)return;camera.aspect=w/h;camera.updateProjectionMatrix();renderer.setSize(w,h);};const observer=new ResizeObserver(resize);observer.observe(host);resize();
  let frame=0;const animate=()=>{frame=requestAnimationFrame(animate);controls.update();renderer.render(scene,camera);};animate();
  active.current={scene,camera,renderer,controls,sun,earth,moon,light,markers,traces,result,initialized:false,view:null};
  return()=>{cancelAnimationFrame(frame);observer.disconnect();controls.dispose();scene.traverse(object=>{if(object instanceof THREE.Mesh||object instanceof THREE.Points||object instanceof THREE.Line){object.geometry.dispose();const materials=Array.isArray(object.material)?object.material:[object.material];materials.forEach(material=>material.dispose());}});pointTexture.dispose();renderer.dispose();renderer.domElement.remove();active.current=null;};
 },[]);
 useEffect(()=>{
  const a=active.current;if(!a)return;
  while(a.result.children.length){const object=a.result.children[0] as THREE.Mesh|THREE.Line;a.result.remove(object);object.geometry.dispose();const materials=Array.isArray(object.material)?object.material:[object.material];materials.forEach(material=>material.dispose());}
  a.result.visible=view==='result-vector'&&resultVector!==null;
  if(a.result.visible&&resultVector){
   a.sun.visible=a.earth.visible=a.moon.visible=a.markers.visible=a.traces.visible=false;
   const direction=toScene(resultVector).normalize(),endpoint=direction.clone().multiplyScalar(8);
   a.result.add(new THREE.Line(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(),endpoint]),new THREE.LineBasicMaterial({color:'#a5f5ca'})));
   const observer=new THREE.Mesh(new THREE.SphereGeometry(.18,24,16),new THREE.MeshBasicMaterial({color:'#60cce2'}));a.result.add(observer);
   const target=new THREE.Mesh(new THREE.SphereGeometry(.24,24,16),new THREE.MeshBasicMaterial({color:'#ffe0a2'}));target.position.copy(endpoint);a.result.add(target);
   const head=new THREE.Mesh(new THREE.ConeGeometry(.13,.38,20),new THREE.MeshBasicMaterial({color:'#a5f5ca'}));head.position.copy(endpoint).addScaledVector(direction,-.42);head.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),direction);a.result.add(head);
   if(!a.initialized||a.view!==view){const midpoint=endpoint.clone().multiplyScalar(.5);let normal=new THREE.Vector3().crossVectors(direction,new THREE.Vector3(0,1,0)).normalize();if(normal.lengthSq()<.01)normal=new THREE.Vector3(0,0,1);a.camera.position.copy(midpoint).addScaledVector(normal,Math.max(22,14/a.camera.aspect));a.controls.target.copy(midpoint);a.camera.lookAt(midpoint);a.controls.update();a.initialized=true;a.view=view;}
   return;
  }
  if(!data){a.sun.visible=a.earth.visible=a.moon.visible=a.markers.visible=a.traces.visible=false;return;}
  const sun=toScene(data.sun),moon=toScene(data.moon);
  const farthest=data.systems.reduce((best,body)=>Math.max(best,toScene(body.position).length()),0);
  const target=view==='earth-moon'?moon:view==='sun-earth'?sun:new THREE.Vector3(farthest,0,0);
  const distance=target.length();if(!Number.isFinite(distance)||distance===0){a.sun.visible=a.earth.visible=a.moon.visible=false;return;}
  const scale=8/distance;
  const origin=view==='solar-system'?sun.clone():new THREE.Vector3();
  a.sun.position.copy(sun).sub(origin).multiplyScalar(scale);a.moon.position.copy(moon).sub(origin).multiplyScalar(scale);a.earth.position.copy(origin).multiplyScalar(-scale);
  const palette=['#aeb7c0','#e4b770','#e87f5c','#d8bc99','#e4d19d','#84d7d9','#708de5','#c9b9a8'];
  const points=data.systems.flatMap(body=>toScene(body.position).multiplyScalar(scale).toArray());points.push(...a.earth.position.toArray(),0,0,0);
  const colors=data.systems.flatMap((_,i)=>new THREE.Color(palette[i]||'#aeeccf').toArray());colors.push(...new THREE.Color('#59d7ed').toArray(),...new THREE.Color('#ffd184').toArray());
  a.markers.geometry.setAttribute('color',new THREE.Float32BufferAttribute(colors,3));
  a.markers.geometry.setAttribute('position',new THREE.Float32BufferAttribute(points,3));a.markers.geometry.computeBoundingSphere();a.markers.visible=view==='solar-system';
  while(a.traces.children.length){const line=a.traces.children[0] as THREE.Line;a.traces.remove(line);line.geometry.dispose();(line.material as THREE.Material).dispose();}
  a.traces.visible=view==='solar-system';
  if(a.traces.visible&&trajectories?.epoch_et===data.epoch_et){
   const traceColors:Record<number,string>={199:'#aeb7c0',299:'#e4b770',399:'#59d7ed',4:'#e87f5c',5:'#d8bc99',6:'#e4d19d',7:'#84d7d9',8:'#708de5',9:'#c9b9a8'};
   for(const track of trajectories.tracks){let segment:THREE.Vector3[]=[];const flush=()=>{if(segment.length>1)a.traces.add(new THREE.Line(new THREE.BufferGeometry().setFromPoints(segment),new THREE.LineBasicMaterial({color:traceColors[track.id]||'#aeeccf',transparent:true,opacity:.45})));segment=[];};
    for(const point of track.samples){if(!point){flush();continue;}segment.push(toScene(point).multiplyScalar(scale));}flush();
   }
  }
  a.sun.scale.setScalar(radii.sun*scale);a.earth.scale.setScalar(radii.earth*scale);a.moon.scale.setScalar(radii.moon*scale);
  a.sun.visible=view!=='earth-moon';a.earth.visible=true;a.moon.visible=true;
  a.light.position.copy(sun).normalize().multiplyScalar(10);
  if(!a.initialized||a.view!==view){
   const midpoint=view==='solar-system'?new THREE.Vector3():target.clone().multiplyScalar(scale/2);const direction=target.clone().normalize();let cameraDirection=new THREE.Vector3().crossVectors(direction,new THREE.Vector3(0,1,0)).normalize();
   if(cameraDirection.lengthSq()<.01)cameraDirection=new THREE.Vector3(0,0,1);
   a.camera.position.copy(midpoint).addScaledVector(cameraDirection,Math.max(22,(view==='solar-system'?28:14)/a.camera.aspect));a.controls.target.copy(midpoint);a.camera.lookAt(midpoint);a.controls.update();a.initialized=true;a.view=view;
  }
 },[data,view,trajectories,resultVector]);
 return <div className="space-canvas" ref={mount} aria-label={view==='result-vector'?'Computed vector in three-dimensional space':'Solar system positions calculated from SPICE; barycenter markers indicate position only'}/>;
}
