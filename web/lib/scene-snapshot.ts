import type {SceneData,TrajectoryData} from '../components/SpaceScene';
export type SceneSnapshot={sequence:number;scene:SceneData|null;trajectories:TrajectoryData|null};
export function commitSceneSnapshot(current:SceneSnapshot,sequence:number,scene:SceneData,trajectories:TrajectoryData|null):SceneSnapshot {
 if(sequence<=current.sequence)return current;
 if(trajectories&&scene.epoch_et!==trajectories.epoch_et)return current;
 if(!trajectories&&current.scene)return current;
 return {sequence,scene,trajectories};
}
