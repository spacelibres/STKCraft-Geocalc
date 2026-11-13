import random,struct,math
r=random.Random(481051)
with open('target/trig-corpus.txt','w') as output:
 for index in range(500000):
  scale=[math.pi,1e6,1e20,1e100,1e300][index%5]
  x=r.uniform(-1,1)*scale
  output.write(struct.pack('>d',x).hex()+'\n')
