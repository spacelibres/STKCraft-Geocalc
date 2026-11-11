import importlib.util,pathlib,unittest
from unittest.mock import patch
p=pathlib.Path(__file__).resolve().parents[1]/'wgc2_geometry_compare.py';spec=importlib.util.spec_from_file_location('geometry_families',p);c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
class NewFamilyCoverage(unittest.TestCase):
 def test_new_families_cannot_pass_empty_local_rows(self):
  reference={'columns':[{'outputID':i} for i in ['DATE','X','Y','Z']],'rows':[[0.,1.,2.,3.]]}
  for name in ['run_sub_observer','run_sub_solar','run_pointing']:
   with self.subTest(name=name),patch.object(c,'reference_result',side_effect=lambda w,p:{'columns':[{'outputID':i} for i in ['DATE']+c.POINTING_OUTPUTS.get(p.get('coordinateRepresentation'),['X','Y','Z'])],'rows':[[0.,1.,2.,3.]]}),patch.object(c,'crate_queries',return_value=[]):
    with self.assertRaisesRegex(RuntimeError,'row count'):getattr(c,name)(None,'-','-')
 def test_pointing_has_explicit_width_and_identity(self):
  for output in [['POSITION','999','399','0','0','0','1','2','3'],['POSITION','301','399','0','0','0','1','2']]:
   with self.assertRaises(RuntimeError):list(c.verified_pairs(['POSITION,301,399,0'],[None],[output],'pointing_full'))
if __name__=='__main__':unittest.main()
