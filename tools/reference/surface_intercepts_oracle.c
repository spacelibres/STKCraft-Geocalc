/* Independent N0067 SINCPT outputs, found flags and derived output primitives. */
#include "SpiceUsr.h"
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <stdlib.h>
static void bits(double x) {uint64_t b;if(!isfinite(x))exit(9);memcpy(&b,&x,sizeof(b));printf("%016llx",(unsigned long long)b);}
static int error(void) {
    char short_message[256],long_message[2048];
    if(!failed_c())return 0;
    getmsg_c("SHORT",sizeof(short_message),short_message);getmsg_c("LONG",sizeof(long_message),long_message);
    printf("E|%s\n",short_message);fprintf(stderr,"NATIVE_ERROR|%s|%s\n",short_message,long_message);reset_c();return 1;
}
int main(void) {
    const char *methods[]={"ELLIPSOID","DSK/UNPRIORITIZED","DSK/UNPRIORITIZED/SURFACES=100","DSK/UNPRIORITIZED/SURFACES=999"};
    const char *corrections[]={"NONE","LT","LT+S","CN","CN+S","XLT","XLT+S","XCN","XCN+S"};
    const char *frames[]={"IAU_MOON","DSK_LOCAL"};
    const char *observers[]={"EARTH","SUN"};
    double epochs[]={0,1e8,5e8},radii[]={3.2,4.1,4.9};
    double offsets[3][3]={{0,0,0},{.3,-.2,.1},{3,3,3}};
    int method,corr,frame,observer,epoch,ray,index;
    fprintf(stderr,"CRT_FMA:%d\n",_get_FMA3_enable());if(_get_FMA3_enable()!=1)return 7;
    erract_c("SET",0,"RETURN");errprt_c("SET",0,"NONE");
    furnsh_c("target/kernels/de440s.bsp");furnsh_c("target/kernels/pck00011.tpc");
    furnsh_c("tests/golden/dsk/frames.tf");furnsh_c("tests/golden/dsk/optimized.bds");
    pdpool_c("BODY301_RADII",3,radii);if(error())return 3;
    for(method=0;method<4;++method)for(corr=0;corr<9;++corr)for(frame=0;frame<2;++frame)
    for(observer=0;observer<2;++observer)for(epoch=0;epoch<3;++epoch)for(ray=0;ray<3;++ray) {
        double direction[3],lt,rotation[3][3],offset[3],point[3],trgepc,vector[3],distance;
        SpiceBoolean found;
        spkpos_c("MOON",epochs[epoch],"J2000",corrections[corr],observers[observer],direction,&lt);
        pxform_c(frames[frame],"J2000",epochs[epoch],rotation);mxv_c(rotation,offsets[ray],offset);vadd_c(direction,offset,direction);
        if(error())return 4;
        printf("%d|%s|%s|%s|",method,corrections[corr],frames[frame],observers[observer]);bits(epochs[epoch]);
        for(index=0;index<3;++index){putchar('|');bits(direction[index]);}putchar('|');
        sincpt_c(methods[method],"MOON",epochs[epoch],frames[frame],corrections[corr],observers[observer],"J2000",direction,point,&trgepc,vector,&found);
        if(error())continue;
        if(!found){puts("MISS");continue;}
        distance=vnorm_c(vector);
        printf("N|9");for(index=0;index<3;++index){putchar('|');bits(point[index]);}
        putchar('|');bits(trgepc);
        for(index=0;index<3;++index){putchar('|');bits(vector[index]);}
        putchar('|');bits(distance);putchar('|');bits(distance/clight_c());putchar('\n');
    }
    kclear_c();return 0;
}
