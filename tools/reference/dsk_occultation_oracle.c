/* Next-milestone independent native GFOCLT capture; no production linkage. */
#include "SpiceUsr.h"
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <math.h>

static void bits(double value) {
    uint64_t encoded;
    if (!isfinite(value)) { puts("NONFINITE"); return; }
    memcpy(&encoded,&value,sizeof(encoded));
    printf("%016llx",(unsigned long long)encoded);
}
static int failed(void) {
    char short_message[256],long_message[2048];
    if (!failed_c()) return 0;
    getmsg_c("SHORT",sizeof(short_message),short_message);
    getmsg_c("LONG",sizeof(long_message),long_message);
    printf("E|%s\n",short_message);
    fprintf(stderr,"NATIVE_ERROR|%s|%s\n",short_message,long_message);
    reset_c();return 1;
}
int main(int argc,char **argv) {
    SpiceInt handle,index,polarity,corr,frame,surface,condition,n,placement;
    double states[2][6]={{20,0,0,0,0,0},{20,0,0,0,0,0}};
    const char *corrections[]={"NONE","LT","LT+S","CN","CN+S","XLT","XLT+S","XCN","XCN+S"};
    const char *frames[]={"IAU_MOON","DSK_LOCAL"};
    const char *surfaces[]={"DSK/UNPRIORITIZED","DSK/UNPRIORITIZED/SURFACES=100","DSK/UNPRIORITIZED/SURFACES=999"};
    const char *conditions[]={"ANY","FULL","PARTIAL","ANNULAR"};
    SPICEDOUBLE_CELL(confinement,200);
    SPICEDOUBLE_CELL(result,200);
    fprintf(stderr,"CRT_FMA:%d\n",_get_FMA3_enable());
    if(argc!=2 || _get_FMA3_enable()!=1) return 2;
    erract_c("SET",0,"RETURN");errprt_c("SET",0,"NONE");
    spkopn_c(argv[1],"independent DSK occultation input",0,&handle);
    spkw08_c(handle,-999100,301,"J2000",-1000,1000,"constant observer",1,2,states,-1000,2000);
    states[0][0]=-20;states[1][0]=-20;
    states[0][1]=-150;states[1][1]=150;
    states[0][4]=.15;states[1][4]=.15;
    spkw08_c(handle,-999101,301,"J2000",-1000,1000,"moving point target",1,2,states,-1000,2000);
    states[0][0]=10;states[1][0]=10;
    spkw08_c(handle,-999102,301,"J2000",-1000,1000,"foreground moving point",1,2,states,-1000,2000);
    spkcls_c(handle);
    if(failed()) return 3;
    furnsh_c("target/kernels/de440s.bsp");
    furnsh_c("target/kernels/pck00011.tpc");
    furnsh_c("tests/golden/dsk/frames.tf");
    furnsh_c("tests/golden/dsk/optimized.bds");
    furnsh_c(argv[1]);
    if(failed()) return 4;
    wninsd_c(-120,120,&confinement);
    for(placement=0;placement<2;++placement)
    for(polarity=0;polarity<2;++polarity)
    for(corr=0;corr<9;++corr)
    for(frame=0;frame<2;++frame)
    for(surface=0;surface<3;++surface)
    for(condition=0;condition<4;++condition) {
        printf("%d|%d|%s|%s|%d|%s|",placement,polarity,corrections[corr],frames[frame],surface,conditions[condition]);
        scard_c(0,&result);
        gfoclt_c(conditions[condition],polarity ? (placement ? "-999102" : "-999101") : "301",
            polarity ? "POINT" : surfaces[surface],polarity ? "" : frames[frame],
            polarity ? "301" : (placement ? "-999102" : "-999101"),polarity ? surfaces[surface] : "POINT",
            polarity ? frames[frame] : "",corrections[corr],"-999100",2.5,&confinement,&result);
        if(failed()) continue;
        n=wncard_c(&result);printf("W|%d",(int)n);
        for(index=0;index<n;++index) {
            double start,stop;
            wnfetd_c(&result,index,&start,&stop);
            putchar('|');bits(start);putchar('|');bits(stop);putchar('|');bits(stop-start);
        }
        putchar('\n');
    }
    kclear_c();return failed() ? 5 : 0;
}
