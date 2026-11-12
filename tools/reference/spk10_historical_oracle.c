/* Independent N0067 historical SPKW10/SPKEZR/SPKEZP regression. */
#include "SpiceUsr.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
static void bits(double value) {
    uint64_t word;
    if(!isfinite(value)){fprintf(stderr,"NONFINITE_NATIVE_OUTPUT\n");exit(9);}
    memcpy(&word,&value,sizeof(word));printf("%016llx",(unsigned long long)word);
}
static int error(void) {
    char short_message[256],long_message[2048];
    if(!failed_c())return 0;
    getmsg_c("SHORT",sizeof(short_message),short_message);
    getmsg_c("LONG",sizeof(long_message),long_message);
    printf("E|%s\n",short_message);fprintf(stderr,"NATIVE_ERROR|%s|%s\n",short_message,long_message);
    reset_c();return 1;
}
int main(int argc,char **argv) {
    const double bases[]={-4000000000.,-2000000000.,-1500000000.,-1000000000.,-900000000.,
        -883656100.,-883656010.,-883656000.,-883655958.,-883655957.816,-883655957.,-883655900.,0.};
    const double periods[]={90.,1436.1};
    const double offsets[]={-10000.,-3600.0000001,-3600.,-100.,0.,100.,3600.,7200.,10000.};
    const char *frames[]={"J2000","B1950","GALACTIC","IAU_EARTH"};
    const char *corrections[]={"NONE","LT","LT+S","CN","CN+S","XLT","XLT+S","XCN","XCN+S"};
    double geophs[]={.001082616,-2.53881e-6,-1.65597e-6,.0743669161,120.,78.,6378.135,1.};
    double query_bases[52];SpiceInt targets[52],handle,index=0,base,orbit,packets,i,time,frame,corr,observer;
    char segment[41],target_name[32];
    fprintf(stderr,"CRT_FMA:%d\n",_get_FMA3_enable());
    if(argc!=2||_get_FMA3_enable()!=1)return 1;
    erract_c("SET",0,"RETURN");errprt_c("SET",0,"NONE");
    furnsh_c("target/kernels/naif0012.tls");
    spkopn_c(argv[1],"native historical SPK10",0,&handle);if(error())return 2;
    for(base=0;base<13;++base)for(orbit=0;orbit<2;++orbit)for(packets=1;packets<=3;packets+=2) {
        double elements[30],epochs[3];SpiceInt body=-931000-index;
        for(i=0;i<packets;++i){
            double *e=&elements[i*10];epochs[i]=bases[base]+(packets==1 ? 0. : (i-1)*3600.);
            e[0]=0.;e[1]=0.;e[2]=1e-6;e[3]=.7+i*.0001;e[4]=.2+i*.001;
            e[5]=orbit==0 ? .02 : .2;e[6]=.3+i*.002;e[7]=.4+i*.003;
            e[8]=2*pi_c()/periods[orbit];e[9]=epochs[i];
        }
        sprintf(segment,"historical epoch target %d",body);
        spkw10_c(handle,body,399,"J2000",epochs[0]-20000.,epochs[packets-1]+20000.,segment,geophs,packets,elements,epochs);
        if(error())return 3;
        targets[index]=body;query_bases[index]=bases[base];++index;
    }
    spkcls_c(handle);if(error())return 4;
    furnsh_c(argv[1]);furnsh_c("target/kernels/de440s.bsp");furnsh_c("target/kernels/pck00011.tpc");if(error())return 5;
    for(index=0;index<52;++index)for(time=0;time<9;++time)for(frame=0;frame<4;++frame)
    for(observer=0;observer<2;++observer)for(corr=0;corr<9;++corr){
        double et=query_bases[index]+offsets[time],state[6],lt,position[3],plt;int j;
        SpiceInt observer_id=observer==0 ? 399 : 301;
        sprintf(target_name,"%d",targets[index]);
        printf("%d|",targets[index]);bits(et);printf("|%s|%s|%d|",frames[frame],corrections[corr],observer_id);
        spkezr_c(target_name,et,frames[frame],corrections[corr],observer==0 ? "399" : "301",state,&lt);
        if(error())continue;
        spkezp_c(targets[index],et,frames[frame],corrections[corr],observer_id,position,&plt);
        if(error())continue;
        printf("N|11");for(j=0;j<6;++j){putchar('|');bits(state[j]);}
        putchar('|');bits(lt);for(j=0;j<3;++j){putchar('|');bits(position[j]);}
        putchar('|');bits(plt);putchar('\n');
    }
    kclear_c();return 0;
}
