/* Independent N0067 LATSRF capture; never linked into production. */
#include "SpiceUsr.h"
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <stdlib.h>

static void bits(double value) {
    uint64_t raw;
    if (!isfinite(value)) exit(9);
    memcpy(&raw,&value,sizeof(raw));
    printf("%016llx",(unsigned long long)raw);
}
static int error(void) {
    char short_message[256],long_message[2048];
    if (!failed_c()) return 0;
    getmsg_c("SHORT",sizeof(short_message),short_message);
    getmsg_c("LONG",sizeof(long_message),long_message);
    printf("E|%s\n",short_message);
    fprintf(stderr,"NATIVE_ERROR|%s|%s\n",short_message,long_message);
    reset_c();return 1;
}
int main(void) {
    const char *files[]={"optimized.bds","dense_lat.bds"};
    const char *methods[]={"ELLIPSOID","DSK/UNPRIORITIZED","DSK/UNPRIORITIZED/SURFACES=100","DSK/UNPRIORITIZED/SURFACES=999"};
    const char *frames[]={"IAU_MOON","DSK_LOCAL"};
    double epochs[]={0,1e8,5e8},radii[]={3.2,4.1,4.9};
    double longitude[]={-4*pi_c(),-pi_c(),-.3,0,.3,pi_c(),4*pi_c()};
    double latitude[]={-pi_c()/2,-1.2,-.1,0,.2,1.2,pi_c()/2};
    int file,method,frame,epoch,lon,lat;
    fprintf(stderr,"CRT_FMA:%d\n",_get_FMA3_enable());
    if (_get_FMA3_enable()!=1) return 7;
    erract_c("SET",0,"RETURN");errprt_c("SET",0,"NONE");
    for(file=0;file<2;++file) {
        char path[256];
        kclear_c();
        furnsh_c("target/kernels/de440s.bsp");
        furnsh_c("target/kernels/pck00011.tpc");
        furnsh_c("tests/golden/dsk/frames.tf");
        snprintf(path,sizeof(path),"tests/golden/dsk/%s",files[file]);furnsh_c(path);
        pdpool_c("BODY301_RADII",3,radii);
        if (error()) return 3;
        for(method=0;method<4;++method)
        for(frame=0;frame<2;++frame)
        for(epoch=0;epoch<3;++epoch)
        for(lon=0;lon<7;++lon)
        for(lat=0;lat<7;++lat) {
            double angles[1][2]={{longitude[lon],latitude[lat]}},point[1][3];
            int index;
            printf("%s|%d|%s|",files[file],method,frames[frame]);bits(epochs[epoch]);
            putchar('|');bits(longitude[lon]);putchar('|');bits(latitude[lat]);putchar('|');
            latsrf_c(methods[method],"MOON",epochs[epoch],frames[frame],1,angles,point);
            if (error()) continue;
            printf("N|3");
            for(index=0;index<3;++index) {putchar('|');bits(point[0][index]);}
            putchar('\n');
        }
    }
    kclear_c();return 0;
}
