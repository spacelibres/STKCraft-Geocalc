import sys,struct,math
for line in sys.stdin:
 x=struct.unpack('>d',int(line.strip(),16).to_bytes(8,'big'))[0]
 print(struct.pack('>d',math.cos(x)).hex())
