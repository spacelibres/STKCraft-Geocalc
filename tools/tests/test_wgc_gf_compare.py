import importlib.util,pathlib,unittest
p=pathlib.Path(__file__).resolve().parents[1]/'wgc_gf_compare.py'
s=importlib.util.spec_from_file_location('gf_compare',p);g=importlib.util.module_from_spec(s);s.loader.exec_module(g)
def result(rows):return {'columns':[{'outputID':'DATE','type':'DATE','name':'Start Time'},{'outputID':'DATE','type':'DATE','name':'Stop Time'},{'outputID':'DURATION','type':'NUMBER','name':'Duration (secs)'}],'rows':rows}
class StrictGfRows(unittest.TestCase):
 def test_exact_all_columns_pass(self):self.assertTrue(g.compare_gf_rows(result([['0','2',2.]]),[['0','2','2']],'gf')['passed'])
 def test_missing_or_extra_rows_fail(self):
  for rows in [[],[['0','2','2'],['4','5','1']]]:
   with self.assertRaisesRegex(RuntimeError,'row count'):g.compare_gf_rows(result([['0','2',2.]]),rows,'gf')
 def test_empty_does_not_establish_endpoint_parity(self):
  with self.assertRaisesRegex(RuntimeError,'zero'):g.compare_gf_rows(result([]),[],'gf')
 def test_nan_and_missing_duration_fail(self):
  for rows in [[['0','2','nan']],[['0','2']]]:
   with self.assertRaises(RuntimeError):g.compare_gf_rows(result([['0','2',2.]]),rows,'gf')
 def test_raw_small_difference_is_not_parity(self):self.assertFalse(g.compare_gf_rows(result([['0','2',2.]]),[['0','2','2.000000001']],'gf')['passed'])
 def test_only_explicit_output_serialization_can_pass(self):
  ref=result([['0.000000','2.000000',2.]])
  self.assertFalse(g.compare_gf_rows(ref,[['0.0000001','2','2']],'gf')['passed'])
  self.assertTrue(g.compare_gf_rows(ref,[['0.0000001','2','2']],'gf',output_exact=True)['passed'])
 def test_column_order_and_date_types_required(self):
  for index,value in [(0,{'outputID':'DURATION','type':'NUMBER','name':'Start Time'}),(1,{'outputID':'DATE','type':'NUMBER','name':'Stop Time'})]:
   r=result([['0','2',2.]]);r['columns'][index]=value
   with self.assertRaisesRegex(RuntimeError,'schema'):g.compare_gf_rows(r,[['0','2','2']],'gf')
 def test_reversed_local_interval_fails(self):
  with self.assertRaises(RuntimeError):g.compare_gf_rows(result([['0','2',2.]]),[['2','0','2']],'gf')
if __name__=='__main__':unittest.main()
class StrictGfProtocol(unittest.TestCase):
 def test_truncated_window_rows_cannot_pass(self):
  with self.assertRaisesRegex(RuntimeError,'count'):g.parse_gf_output('BEGIN,a,2\nROW,a,0,0,1,1\nEND,a\n',['a'])
 def test_identity_and_row_index_required(self):
  for text in ['BEGIN,b,0\nEND,b\n','BEGIN,a,1\nROW,a,1,0,1,1\nEND,a\n']:
   with self.assertRaises(RuntimeError):g.parse_gf_output(text,['a'])
 def test_missing_query_cannot_pass(self):
  with self.assertRaisesRegex(RuntimeError,'query'):g.parse_gf_output('BEGIN,a,0\nEND,a\n',['a','b'])
 def test_explicit_calculation_error_remains_failure(self):
  self.assertEqual(g.parse_gf_output('ERROR,a,missing kernel\n',['a'])['a']['error'],'missing kernel')
