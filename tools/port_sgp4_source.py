"""Mechanical scalar AST port of official N0067 SGP4 C. No C in production."""
from pathlib import Path
import re
from pycparser import c_parser,c_ast as A
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'target/reference/cached-n0067/cspice/src/cspice'
NAMES=['zzinil','zzdscm','zzdsin','zzdspr','zzdspc','zzsgp4']
FUNS={};GLOBALS={};OUT={};PARAM={}
def typename(t):
 while not isinstance(t,A.IdentifierType):t=t.type
 return {'doublereal':'f64','integer':'i32','logical':'bool','int':'i32','double':'f64','ftnlen':'i32'}[t.names[0]]
def source(n):
 s=(BASE/(n+'.c')).read_text();s=re.sub(r'/\*.*?\*/','',s,flags=re.S);s=re.sub(r'^#.*','',s,flags=re.M)
 if n=='zzsgp4':
  before=s[:s.index('    if (geophs)')];init=s[s.index('L_xxsgp4i:')+len('L_xxsgp4i:'):s.index('L_xxsgp4e:')];init=re.sub(r'return 0;\s*$','',init)
  ev=s[s.index('L_xxsgp4e:')+len('L_xxsgp4e:'):s.index('\n int zzsgp4_(')];s=before+init+ev
 ast=c_parser.CParser().parse('typedef double doublereal;typedef int integer;typedef int logical;typedef int ftnlen;'+s)
 for x in ast.ext:
  if isinstance(x,A.Decl) and x.init is not None:GLOBALS[x.name]=(typename(x.type),x.init)
 return next(x for x in ast.ext if isinstance(x,A.FuncDef))
for n in NAMES:FUNS[n]=source(n)
class Writes(A.NodeVisitor):
 def __init__(self):self.ids=set()
 def visit_Assignment(self,n):
  if isinstance(n.lvalue,A.UnaryOp) and n.lvalue.op=='*':self.ids.add(n.lvalue.expr.name)
  self.generic_visit(n)
for name,f in FUNS.items():
 args=f.decl.type.args.params;PARAM[name]=args;v=Writes();v.visit(f.body);OUT[name]=[p for p in args if p.name in v.ids]
class Port:
 def __init__(self,name):
  self.name=name;self.types={k:v[0] for k,v in GLOBALS.items()};self.types.update({'TRUE_':'bool','FALSE_':'bool'});self.params=PARAM[name]
  for p in self.params:self.types[p.name]=typename(p.type)
  for d in FUNS[name].body.block_items:
   if isinstance(d,A.Decl) and not isinstance(d.type,A.FuncDecl):self.types[d.name]=typename(d.type)
 def typ(self,n):
  if isinstance(n,A.ID):return self.types[n.name]
  if isinstance(n,A.Constant):return 'f64' if n.type=='double' else 'i32'
  if isinstance(n,A.ArrayRef):return self.typ(n.name)
  if isinstance(n,A.Cast):return typename(n.to_type)
  if isinstance(n,A.UnaryOp):return 'bool' if n.op=='!' else self.typ(n.expr)
  if isinstance(n,A.Assignment):return self.typ(n.lvalue)
  if isinstance(n,A.ExprList):return self.typ(n.exprs[-1])
  if isinstance(n,A.BinaryOp):
   if n.op in ['<','>','<=','>=','==','!=','&&','||']:return 'bool'
   return 'f64' if 'f64' in [self.typ(n.left),self.typ(n.right)] else 'i32'
  if isinstance(n,A.FuncCall):return 'bool' if n.name.name in ['failed_','return_'] else 'f64'
  raise TypeError(('type',type(n),n))
 def cast(self,n,t):
  s=self.e(n)
  if self.typ(n)!=t:s=f'({s} as {t})'
  return s
 def e(self,n):
  if isinstance(n,A.ID):return {'TRUE_':'true','FALSE_':'false'}.get(n.name,n.name)
  if isinstance(n,A.Constant):
   if n.type=='string':return n.value
   s=n.value
   if s.startswith('.'):s='0'+s
   if s.endswith('.'):s+='0'
   return s
  if isinstance(n,A.ArrayRef):return f'{self.e(n.name)}[{self.cast(n.subscript,"usize")}]'
  if isinstance(n,A.Cast):return f'({self.e(n.expr)} as {typename(n.to_type)})'
  if isinstance(n,A.UnaryOp):
   if n.op in ['*','&']:return self.e(n.expr)
   if n.op in ['++','p++']:return f'{{{self.e(n.expr)} += 1; {self.e(n.expr)}}}'
   return f'({n.op}{self.e(n.expr)})'
  if isinstance(n,A.Assignment):return '{'+self.assign(n)+' '+self.e(n.lvalue)+'}'
  if isinstance(n,A.ExprList):return '{'+' '.join(self.st(x) for x in n.exprs[:-1])+' '+self.e(n.exprs[-1])+'}'
  if isinstance(n,A.BinaryOp):
   lt,rt=self.typ(n.left),self.typ(n.right);t='f64' if 'f64' in [lt,rt] else lt
   return f'({self.cast(n.left,t)} {n.op} {self.cast(n.right,t)})'
  if isinstance(n,A.FuncCall):
   name=n.name.name;args=n.args.exprs if n.args else [];a=[self.e(x) for x in args]
   if name in ['sin','cos','sqrt','abs']:return f'({a[0]}).{name}()'
   if name=='atan2':return f'({a[0]}).atan2({a[1]})'
   if name=='pow_dd':return f'({a[0]}).powf({a[1]})'
   if name=='d_mod':return f'f2c_mod({a[0]}, {a[1]})'
   if name=='d_int':return f'({a[0]}).trunc()'
   if name=='d_sign':return f'if {a[1]} >= 0.0 {{({a[0]}).abs()}} else {{-({a[0]}).abs()}}'
   if name=='twopi_':return 'std::f64::consts::TAU'
   if name=='pi_':return 'std::f64::consts::PI'
   raise ValueError(('call',name))
  raise TypeError(('expr',type(n),n))
 def assign(self,n):return f'{self.e(n.lvalue)} {n.op} {self.cast(n.rvalue,self.typ(n.lvalue))};'
 def result(self):
  if self.name=='zzsgp4':return 'Ok(state)'
  return 'Ok(('+''.join(p.name+',' for p in OUT[self.name])+'))'
 def st(self,n):
  if n is None:return ''
  if isinstance(n,A.Compound):return '{\n'+ '\n'.join(self.st(x) for x in n.block_items or [])+'\n}'
  if isinstance(n,A.Decl):
   if isinstance(n.type,A.FuncDecl) or 'extern' in n.storage:return ''
   typ=self.types[n.name];default='false' if typ=='bool' else '0.0' if typ=='f64' else '0'
   if isinstance(n.type,A.ArrayDecl):typ=f'[{typ};{n.type.dim.value}]';default=f'[{default};{n.type.dim.value}]'
   return f'let mut {n.name}: {typ} = {self.e(n.init) if n.init else default};'
  if isinstance(n,A.Assignment):return self.assign(n)
  if isinstance(n,A.ExprList):return '\n'.join(self.st(x) for x in n.exprs)
  if isinstance(n,A.If):
   if isinstance(n.cond,A.FuncCall) and n.cond.name.name in ['return_','failed_']:return ''
   return f'if {self.e(n.cond)} {self.block(n.iftrue)}'+(f' else {self.block(n.iffalse)}' if n.iffalse else '')
  if isinstance(n,A.While):return f'while {self.e(n.cond)} {self.block(n.stmt)}'
  if isinstance(n,A.For):return self.st(n.init)+f'\nwhile {self.e(n.cond)} {{'+self.st(n.stmt)+self.st(n.next)+'}'
  if isinstance(n,A.UnaryOp):return self.e(n)+';'
  if isinstance(n,A.Return):return f'return {self.result()};'
  if isinstance(n,A.FuncCall):
   name=n.name.name;args=n.args.exprs if n.args else []
   if name in ['chkin_','chkout_','setmsg_','errdp_','errint_']:return ''
   if name=='sigerr_':return f'return Err(GeoError::BadSearch({args[0].value}));'
   if name=='ttrans_':return 'epoch = epoch_days;'
   if name in [x+'_' for x in NAMES[:-1]]:
    fun=name[:-1];outs=OUT[fun];params=PARAM[fun];a=[];targets=[]
    for p,x in zip(params,args):
     if p in outs:targets.append(self.e(x))
     if p.name=='geophs':a.append(self.e(x))
     else:a.append(self.cast(x,typename(p.type)))
    return '('+''.join(x+',' for x in targets)+') = '+fun+'('+','.join(a)+')?;'
   raise ValueError(('stmtcall',name))
  raise TypeError(('stmt',type(n),n))
 def block(self,n):return self.st(n) if isinstance(n,A.Compound) else '{'+self.st(n)+'}'
 def fn(self):
  if self.name=='zzsgp4':
   sig='pub(crate) fn propagate(geophs:&[f64;8],elems:&[f64;10],mut opmode:i32,mut t:f64,epoch_days:f64)->Result<[f64;6],GeoError>'
   body=self.st(FUNS[self.name].body)
   body=body.replace('epoch = (tvec[(0 as usize)] - 2433281.5);','epoch = epoch_days;').replace('tvec[(0 as usize)] = elems[(9 as usize)];','')
   body=body.replace('epoch = epoch_days;\nepoch = epoch_days;','epoch = epoch_days;')
   return sig+' { let mut state=[0.0;6];\n'+body[1:]
  p=[]
  for x in self.params:p.append(('geophs:&[f64;8]' if x.name=='geophs' else f'mut {x.name}:{typename(x.type)}'))
  result='('+''.join(typename(x.type)+',' for x in OUT[self.name])+')'
  return 'fn '+self.name+'('+','.join(p)+')->Result<'+result+',GeoError>'+self.st(FUNS[self.name].body)
out='//! Source-order CSPICE N0067 SGP4 in TEME coordinates.\n//! The caller supplies JDUTC-2433281.5 explicitly; no native runtime.\n#![allow(unused_mut,unused_variables,unused_assignments,unreachable_code,unused_parens,non_snake_case,non_upper_case_globals)]\nuse crate::error::GeoError;\n'
out+='fn f2c_mod(x:f64,y:f64)->f64 { x-y*(x/y).trunc() }\n'
for k,(t,v) in GLOBALS.items():out+=f'const {k}:{t} = '+Port('zzinil').e(v)+';\n'
for n in NAMES:out+=Port(n).fn()+'\n'
p=ROOT/'src/ephemeris/sgp4.rs';old=p.read_text(encoding='utf-8');tests=old[old.index('#[cfg(test)]'):];p.write_text(out+tests,encoding='utf-8')
import subprocess
subprocess.run(['rustfmt','--edition','2021',str(p)],check=True)
