"""Test propagation of one platform cosine bit into the captured rotations."""
import json, math
w=[json.loads(x) for x in open('docs/reference/native-cross-platform/orientation-windows.jsonl')][1]
l=[json.loads(x) for x in open('docs/reference/native-cross-platform/orientation-linux.jsonl')][1]
wm=json.load(open('docs/reference/native-cross-platform/sub-tangent-windows.json'))[-2]['matrix_bits']
lm=json.load(open('docs/reference/native-cross-platform/sub-tangent-linux.json'))[-2]['matrix_bits']
ra,dec,meridian=map(float.fromhex,w['angles'][:3])
def mul(a,b):return [[a[i][0]*b[0][j]+a[i][1]*b[1][j]+a[i][2]*b[2][j] for j in range(3)] for i in range(3)]
def r3(a):return [[math.cos(a),math.sin(a),0],[-math.sin(a),math.cos(a),0],[0,0,1]]
def r1(a):return [[1,0,0],[0,math.cos(a),math.sin(a)],[0,-math.sin(a),math.cos(a)]]
def calc(c):
    a=r3(meridian);a[0][0]=a[1][1]=float.fromhex(c)
    return [x.hex() for row in mul(a,mul(r1(math.pi/2-dec),r3(math.pi/2+ra))) for x in row]
assert w['angles'] == l['angles']
assert calc(w['cos'][2]) == wm
assert calc(l['cos'][2]) == lm
assert [i for i in range(9) if wm[i] != lm[i]] == [0, 4, 5]
print('Captured rotations reproduced exactly; changing the cosine changes entries 0, 4, 5.')
