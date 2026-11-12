/* Independent N0067 text-kernel and frame oracle; never linked into production. */
#include "SpiceUsr.h"
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void hex_string(const char *text) {
    const unsigned char *p = (const unsigned char *)text;
    while (*p) printf("%02x", (unsigned int)*p++);
}

static void hex_double(double value) {
    uint64_t bits;
    if (!isfinite(value)) exit(9);
    memcpy(&bits, &value, sizeof(bits));
    printf("%016llx", (unsigned long long)bits);
}

static int error_reply(void) {
    char short_message[256], long_message[2048];
    if (!failed_c()) return 0;
    getmsg_c("SHORT", sizeof(short_message), short_message);
    getmsg_c("LONG", sizeof(long_message), long_message);
    printf("E|%s\n", short_message);
    fprintf(stderr, "NATIVE_ERROR|%s|", short_message);
    {
        const unsigned char *p = (const unsigned char *)long_message;
        while (*p) fprintf(stderr, "%02x", (unsigned int)*p++);
    }
    fputc('\n', stderr);
    reset_c();
    return 1;
}

int main(void) {
    char line[16384], *part[8];
    double numbers[512], matrix[6][6], rotation[3][3], epoch;
    char strings[512][256], kind[2], shape[256], frame[256];
    int count, i, j;
    SpiceInt n, center, frame_class, class_id, frame_id;
    SpiceBoolean found;
    unsigned long long epoch_bits;
    fprintf(stderr, "CRT_FMA:%d\n", _get_FMA3_enable());
    if (_get_FMA3_enable() != 1) return 7;
    erract_c("SET", 0, "RETURN");
    errprt_c("SET", 0, "NONE");
    while (fgets(line, sizeof(line), stdin)) {
        if (!strchr(line, '\n') && !feof(stdin)) return 2;
        line[strcspn(line, "\r\n")] = '\0';
        count = 0;
        part[count++] = strtok(line, "|");
        while (count < 8 && (part[count] = strtok(NULL, "|"))) ++count;
        if (!part[0]) return 3;
        if (!strcmp(part[0], "C") && count == 1) {
            kclear_c();
            if (!error_reply()) puts("OK");
        } else if (!strcmp(part[0], "L") && count == 2) {
            furnsh_c(part[1]);
            if (!error_reply()) puts("OK");
        } else if (!strcmp(part[0], "D") && count == 2) {
            dtpool_c(part[1], &found, &n, kind);
            if (error_reply()) continue;
            if (!found) { puts("MISSING"); continue; }
            if (n < 1 || n > 512) return 4;
            if (kind[0] == 'N') {
                gdpool_c(part[1], 0, 512, &n, numbers, &found);
                if (error_reply()) continue;
                printf("N|%d", (int)n);
                for (i = 0; i < n; ++i) { putchar('|'); hex_double(numbers[i]); }
            } else {
                gcpool_c(part[1], 0, 512, 256, &n, strings, &found);
                if (error_reply()) continue;
                printf("S|%d", (int)n);
                for (i = 0; i < n; ++i) { putchar('|'); hex_string(strings[i]); }
            }
            putchar('\n');
        } else if ((!strcmp(part[0], "R") || !strcmp(part[0], "S")) && count == 4) {
            if (sscanf(part[3], "%llx", &epoch_bits) != 1) return 5;
            memcpy(&epoch, &epoch_bits, sizeof(epoch));
            if (!strcmp(part[0], "R")) {
                pxform_c(part[1], part[2], epoch, rotation);
                if (error_reply()) continue;
                printf("N|9");
                for (i = 0; i < 3; ++i) for (j = 0; j < 3; ++j) {
                    putchar('|'); hex_double(rotation[i][j]);
                }
            } else {
                sxform_c(part[1], part[2], epoch, matrix);
                if (error_reply()) continue;
                printf("N|36");
                for (i = 0; i < 6; ++i) for (j = 0; j < 6; ++j) {
                    putchar('|'); hex_double(matrix[i][j]);
                }
            }
            putchar('\n');
        } else if (!strcmp(part[0], "N") && count == 2) {
            namfrm_c(part[1], &frame_id);
            if (!error_reply()) printf("ID|%d\n", (int)frame_id);
        } else if (!strcmp(part[0], "I") && count == 2) {
            /* An unknown frame must not expose indeterminate C output slots. */
            center = frame_class = class_id = 0;
            frinfo_c(atoi(part[1]), &center, &frame_class, &class_id, &found);
            if (!error_reply()) printf("I|%d|%d|%d|%d\n", (int)found,
                                       (int)center, (int)frame_class, (int)class_id);
        } else if (!strcmp(part[0], "B") && count == 3) {
            bodvrd_c(part[1], part[2], 512, &n, numbers);
            if (error_reply()) continue;
            printf("N|%d", (int)n);
            for (i = 0; i < n; ++i) { putchar('|'); hex_double(numbers[i]); }
            putchar('\n');
        } else if (!strcmp(part[0], "F") && count == 2) {
            double boresight[3], bounds[512][3];
            getfov_c(atoi(part[1]), 512, 256, 256, shape, frame, boresight, &n, bounds);
            if (error_reply()) continue;
            printf("F|"); hex_string(shape); putchar('|'); hex_string(frame);
            printf("|%d", (int)n);
            for (i = 0; i < 3; ++i) { putchar('|'); hex_double(boresight[i]); }
            for (i = 0; i < n; ++i) for (j = 0; j < 3; ++j) {
                putchar('|'); hex_double(bounds[i][j]);
            }
            putchar('\n');
        } else return 6;
    }
    kclear_c();
    return ferror(stdin) || ferror(stdout) ? 8 : 0;
}
