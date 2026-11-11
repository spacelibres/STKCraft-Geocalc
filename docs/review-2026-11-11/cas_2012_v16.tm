KPL/MK

   This meta-kernel lists the CASSINI SPICE kernels providing coverage
   for 2012. All of the kernels listed below are archived in the CAS
   SPICE data set (DATA_SET_ID = "CO-S/J/E/V-SPICE-6-V1.0"). This set
   of files and the order in which they are listed were picked to
   provide the best available data and the most complete coverage for
   the specified year based on the information about the kernels
   available at the time this meta-kernel was made. For detailed
   information about the kernels listed below refer to the internal
   comments included in the kernels and the documentation accompanying
   the CASSINI SPICE data set.

   WARNING: the list of kernels in this meta-kernel includes the latest
   generic PCK file available at the time when this meta-kernel was
   created. This PCK file does not provide all parameters included in
   the CASSINI-specific PCKs and, for some parameters present in this
   file and CASSINI-specific PCKs, provides different values compared
   to the values in the CASSINI-specific PCKs. It is the user's
   responsibility to identify whether a CASSINI-specific PCK(s) is(are)
   needed for his/her application and to use this(these) PCK(s) in
   addition to or instead of the generic PCK file in this meta-kernel.

   It is recommended that users make a local copy of this file and
   modify the value of the PATH_VALUES keyword to point to the actual
   location of the CAS SPICE data set's ``data'' directory on their
   system. Replacing ``/'' with ``\'' and converting line terminators
   to the format native to the user's system may also be required if
   this meta-kernel is to be used on a non-UNIX workstation.

   This file was created on June 10, 2026 by Alyssa Bailey,
   NAIF/JPL. The original name of this file was cas_2012_v16.tm.

   \begindata

      PATH_VALUES     = ( './data' )

      PATH_SYMBOLS    = ( 'KERNELS' )

      KERNELS_TO_LOAD = (

                          '$KERNELS/lsk/naif0012.tls'

                          '$KERNELS/pck/pck00010.tpc'
                          '$KERNELS/pck/enceladus_ssd_250728_v2.tpc'

                          '$KERNELS/fk/cas_rocks_v18.tf'

                          '$KERNELS/fk/cas_mimi_v202.tf'

                          '$KERNELS/fk/cas_dyn_v03.tf'

                          '$KERNELS/fk/cas_v44.tf'

                          '$KERNELS/ik/cas_caps_v03.ti'
                          '$KERNELS/ik/cas_cda_v01.ti'
                          '$KERNELS/ik/cas_cirs_v10.ti'
                          '$KERNELS/ik/cas_inms_v02.ti'
                          '$KERNELS/ik/cas_iss_v10.ti'
                          '$KERNELS/ik/cas_mag_v01.ti'
                          '$KERNELS/ik/cas_mimi_v11.ti'
                          '$KERNELS/ik/cas_radar_v11.ti'
                          '$KERNELS/ik/cas_rpws_v01.ti'
                          '$KERNELS/ik/cas_rss_v03.ti'
                          '$KERNELS/ik/cas_sru_v02.ti'
                          '$KERNELS/ik/cas_uvis_v07.ti'
                          '$KERNELS/ik/cas_vims_v06.ti'

                          '$KERNELS/sclk/cas00172.tsc'

                          '$KERNELS/spk/180927AP_RE_90165_18018.bsp'
                          '$KERNELS/spk/140809BP_IRRE_00256_25017.bsp'
                          '$KERNELS/spk/sat427.bsp'

                          '$KERNELS/spk/200128RU_SCPSE_11357_12016.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12016_12060.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12060_12077.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12077_12093.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12093_12112.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12112_12126.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12126_12151.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12151_12199.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12199_12257.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12257_12280.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12280_12328.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12328_12365.bsp'
                          '$KERNELS/spk/200128RU_SCPSE_12365_13038.bsp'

                          '$KERNELS/ck/12001_13001pa_gapfill_v14.bc'

                          '$KERNELS/ck/11320_12024py_as_flown.bc'
                          '$KERNELS/ck/12024_12097py_as_flown.bc'
                          '$KERNELS/ck/12097_12170py_as_flown.bc'
                          '$KERNELS/ck/12170_12238py_as_flown.bc'
                          '$KERNELS/ck/12238_12307py_as_flown.bc'
                          '$KERNELS/ck/12307_13013py_as_flown.bc'

                          '$KERNELS/ck/11319_11324ra.bc'
                          '$KERNELS/ck/11324_11329ra.bc'
                          '$KERNELS/ck/11329_11334ra.bc'
                          '$KERNELS/ck/11334_11339ra.bc'
                          '$KERNELS/ck/11339_11344ra.bc'
                          '$KERNELS/ck/11344_11349ra.bc'
                          '$KERNELS/ck/11349_11354ra.bc'
                          '$KERNELS/ck/11354_11359ra.bc'
                          '$KERNELS/ck/11359_11364ra.bc'
                          '$KERNELS/ck/11364_12004ra.bc'
                          '$KERNELS/ck/12004_12009ra.bc'
                          '$KERNELS/ck/12009_12014ra.bc'
                          '$KERNELS/ck/12014_12019ra.bc'
                          '$KERNELS/ck/12019_12024ra.bc'
                          '$KERNELS/ck/12024_12029ra.bc'
                          '$KERNELS/ck/12029_12034ra.bc'
                          '$KERNELS/ck/12034_12039ra.bc'
                          '$KERNELS/ck/12039_12044ra.bc'
                          '$KERNELS/ck/12044_12049ra.bc'
                          '$KERNELS/ck/12049_12054ra.bc'
                          '$KERNELS/ck/12054_12059ra.bc'
                          '$KERNELS/ck/12059_12064ra.bc'
                          '$KERNELS/ck/12064_12069ra.bc'
                          '$KERNELS/ck/12069_12074ra.bc'
                          '$KERNELS/ck/12074_12079ra.bc'
                          '$KERNELS/ck/12079_12084ra.bc'
                          '$KERNELS/ck/12084_12089ra.bc'
                          '$KERNELS/ck/12089_12094ra.bc'
                          '$KERNELS/ck/12094_12099ra.bc'
                          '$KERNELS/ck/12099_12104ra.bc'
                          '$KERNELS/ck/12104_12109ra.bc'
                          '$KERNELS/ck/12109_12114ra.bc'
                          '$KERNELS/ck/12114_12119ra.bc'
                          '$KERNELS/ck/12119_12124ra.bc'
                          '$KERNELS/ck/12124_12129ra.bc'
                          '$KERNELS/ck/12129_12134ra.bc'
                          '$KERNELS/ck/12134_12139ra.bc'
                          '$KERNELS/ck/12139_12144ra.bc'
                          '$KERNELS/ck/12144_12149ra.bc'
                          '$KERNELS/ck/12149_12154ra.bc'
                          '$KERNELS/ck/12154_12159ra.bc'
                          '$KERNELS/ck/12159_12164ra.bc'
                          '$KERNELS/ck/12164_12169ra.bc'
                          '$KERNELS/ck/12169_12174ra.bc'
                          '$KERNELS/ck/12174_12179ra.bc'
                          '$KERNELS/ck/12179_12184ra.bc'
                          '$KERNELS/ck/12184_12189ra.bc'
                          '$KERNELS/ck/12189_12194ra.bc'
                          '$KERNELS/ck/12194_12199rb.bc'
                          '$KERNELS/ck/12199_12204ra.bc'
                          '$KERNELS/ck/12204_12209ra.bc'
                          '$KERNELS/ck/12209_12214ra.bc'
                          '$KERNELS/ck/12214_12219ra.bc'
                          '$KERNELS/ck/12219_12224ra.bc'
                          '$KERNELS/ck/12224_12229ra.bc'
                          '$KERNELS/ck/12229_12234ra.bc'
                          '$KERNELS/ck/12234_12239ra.bc'
                          '$KERNELS/ck/12239_12244ra.bc'
                          '$KERNELS/ck/12244_12249rb.bc'
                          '$KERNELS/ck/12249_12254ra.bc'
                          '$KERNELS/ck/12254_12259ra.bc'
                          '$KERNELS/ck/12259_12264ra.bc'
                          '$KERNELS/ck/12264_12269ra.bc'
                          '$KERNELS/ck/12269_12274ra.bc'
                          '$KERNELS/ck/12274_12279rb.bc'
                          '$KERNELS/ck/12279_12284ra.bc'
                          '$KERNELS/ck/12284_12289ra.bc'
                          '$KERNELS/ck/12289_12294ra.bc'
                          '$KERNELS/ck/12294_12299ra.bc'
                          '$KERNELS/ck/12299_12304ra.bc'
                          '$KERNELS/ck/12304_12309ra.bc'
                          '$KERNELS/ck/12309_12314ra.bc'
                          '$KERNELS/ck/12314_12319ra.bc'
                          '$KERNELS/ck/12319_12324ra.bc'
                          '$KERNELS/ck/12324_12329ra.bc'
                          '$KERNELS/ck/12329_12334ra.bc'
                          '$KERNELS/ck/12334_12339ra.bc'
                          '$KERNELS/ck/12339_12344ra.bc'
                          '$KERNELS/ck/12344_12349ra.bc'
                          '$KERNELS/ck/12349_12354ra.bc'
                          '$KERNELS/ck/12354_12359ra.bc'
                          '$KERNELS/ck/12359_12364ra.bc'
                          '$KERNELS/ck/12364_13003ra.bc'
                          '$KERNELS/ck/13003_13008ra.bc'
                          '$KERNELS/ck/13008_13013ra.bc'
                          '$KERNELS/ck/13013_13018ra.bc'

                          '$KERNELS/ck/cas_cda_20130314.bc'

                          '$KERNELS/ck/cas_lemms_05109_20001_v2.bc'

                          '$KERNELS/ck/cas_caps_12076_12091_v2.bc'
                          '$KERNELS/ck/cas_caps_12092_12122_v2.bc'
                          '$KERNELS/ck/cas_caps_12122_12152_v2.bc'
                          '$KERNELS/ck/cas_caps_12153_12154_v1.bc'

                          '$KERNELS/dsk/cas_enceladus_ssd_spc_0128icq_v2.bds'

                        )

   \begintext
