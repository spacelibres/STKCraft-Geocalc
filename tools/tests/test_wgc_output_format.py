import importlib.util,json,pathlib,unittest
from decimal import Decimal
p=pathlib.Path(__file__).resolve().parents[1]/'wgc2_geometry_compare.py';spec=importlib.util.spec_from_file_location('format_compare',p);c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
class WgcOutputFormat(unittest.TestCase):
 def test_actual_nasa_controlled_vector_outputs(self):
  records=json.loads((pathlib.Path(__file__).parent/'fixtures/wgc-output-format.json').read_text(),parse_float=Decimal)
  self.assertEqual(len(records),7)
  for record in records:
   d=record['request']['direction'];reference=record['result']['rows'][0][1:]
   for axis,expected in zip('XYZ',reference):
    with self.subTest(input=d['directionVector'+axis],expected=expected):
     self.assertEqual(c.wgc_output_value(d['directionVector'+axis]),Decimal(str(expected)))
 def test_date_columns_have_declared_six_decimal_places(self):
  self.assertEqual(c.wgc_output_value('1.23456789','TIME_AT_POINT'),Decimal('1.234568'))
 def test_output_compatibility_is_an_explicit_mode(self):
  self.assertFalse(c.WGC_OUTPUT_EXACT)
  with self.assertRaisesRegex(RuntimeError,'raw difference'):c.strict_difference('1.234567891','1.23456789')
if __name__=='__main__':unittest.main()
