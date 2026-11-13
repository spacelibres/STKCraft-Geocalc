import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const cases = JSON.parse(await readFile(new URL('../lib/examples.json',import.meta.url),'utf8'));
const required=['state_vector','angular_separation','angular_size','frame_transformation','illumination_angles','phase_angle','pointing_direction','sub_solar_point','sub_observer_point','surface_intercept_point','tangent_point','osculating_elements','planetocentric_solar_longitude','gf_coordinate_search','gf_angular_separation_search','gf_distance_search','gf_sub_point_search','gf_occultation_search','gf_surface_intercept_point_search','gf_target_in_instrument_fov_search','gf_ray_in_fov_search','gf_range_rate_search','gf_phase_angle_search','gf_illumination_angles_search'];
test('each geometry and finder operation has an evidence-linked selectable example',()=>{
 for(const operation of required){const example=cases.find(item=>item.operation===operation);assert.ok(example,`${operation} missing`);assert.ok(example.title&&example.explanation&&example.source);assert.ok(example.parameters&&typeof example.parameters==='object');if(operation.startsWith('gf_'))assert.ok(example.window&&example.window.end>example.window.start,`${operation} missing window`);}
 assert.equal(new Set(cases.map(item=>item.id)).size,cases.length);
});
