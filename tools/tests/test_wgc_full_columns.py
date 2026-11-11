import importlib.util
import pathlib
import unittest
p=pathlib.Path(__file__).resolve().parents[1]/'wgc2_geometry_compare.py'
spec=importlib.util.spec_from_file_location('full_compare',p);c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
class FullColumns(unittest.TestCase):
 def test_gm_or_other_omitted_reference_column_fails_coverage(self):
  with self.assertRaisesRegex(RuntimeError,'coverage'):
   c.compare_mapped_row(['DATE','X','GM'],[0.,1.,2.],['1'],{'X':0},'elements')
 def test_every_mapped_numeric_cell_is_compared_strictly(self):
  with self.assertRaisesRegex(RuntimeError,'GM'):
   c.compare_mapped_row(['DATE','X','GM'],[0.,1.,2.],['1','2.00000000001'],{'X':0,'GM':1},'elements')
 def test_string_metadata_is_compared_exactly(self):
  with self.assertRaisesRegex(RuntimeError,'LTST'):
   c.compare_mapped_row(['DATE','LTST'],[0.,'12:01:02'],['12:01:03'],{'LTST':0},'illum')
 def test_mapped_row_rejects_unknown_string_column(self):
  with self.assertRaises(RuntimeError):
   c.compare_mapped_row(['DATE','X'],[0.,'not a number'],['not a number'],{'X':0},'state')
 def test_exact_all_column_result_succeeds(self):
  self.assertEqual(c.compare_mapped_row(['DATE','X','LTST'],[0.,1.,'12:01:02'],['1','12:01:02'],{'X':0,'LTST':1},'surface'),0.)
if __name__=='__main__':unittest.main()
