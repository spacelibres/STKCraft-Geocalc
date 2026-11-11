/* Independent binary64 oracle. This file is never linked into production. */
#include "SpiceUsr.h"
#include <fcntl.h>
#include <io.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    static const char *operations[] = {
        "asin", "acos", "atan2", "cos", "cosh", "exp", "log",
        "pow", "sin", "sinh", "sqrt", "tan", "sin_cos", "vsep", "prop2b"
    };
    int operation = -1, width, output_width;
    double input[8], output[6];
    size_t i;
    if (argc != 2) return 5;
    for (i = 0; i < sizeof(operations) / sizeof(operations[0]); ++i)
        if (strcmp(argv[1], operations[i]) == 0) operation = (int)i;
    if (operation < 0) return 5;
    if (_setmode(_fileno(stdin), _O_BINARY) == -1 ||
        _setmode(_fileno(stdout), _O_BINARY) == -1) return 6;
    fprintf(stderr, "CRT_FMA:%d\n", _get_FMA3_enable());
    /* Never force dispatch to obtain a passing capture. */
    if (_get_FMA3_enable() != 1) return 7;
    erract_c("SET", 0, "RETURN");
    errprt_c("SET", 0, "NONE");
    width = operation == 13 ? 6 : operation == 14 ? 8 : 2;
    output_width = operation == 12 ? 2 : operation == 14 ? 6 : 1;
    for (;;) {
        size_t count = fread(input, 1, width * sizeof(double), stdin);
        if (count == 0 && feof(stdin)) break;
        if (count != (size_t)width * sizeof(double)) return 2;
        switch (operation) {
        case 0: output[0] = asin(input[0]); break;
        case 1: output[0] = acos(input[0]); break;
        case 2: output[0] = atan2(input[0], input[1]); break;
        case 3: output[0] = cos(input[0]); break;
        case 4: output[0] = cosh(input[0]); break;
        case 5: output[0] = exp(input[0]); break;
        case 6: output[0] = log(input[0]); break;
        case 7: output[0] = pow(input[0], input[1]); break;
        case 8: output[0] = sin(input[0]); break;
        case 9: output[0] = sinh(input[0]); break;
        case 10: output[0] = sqrt(input[0]); break;
        case 11: output[0] = tan(input[0]); break;
        case 12: output[0] = sin(input[0]); output[1] = cos(input[0]); break;
        case 13: output[0] = vsep_c(input, input + 3); break;
        case 14: prop2b_c(input[0], input + 1, input[7], output); break;
        default: return 5;
        }
        if (failed_c()) return 3;
        if (fwrite(output, sizeof(double), output_width, stdout) !=
            (size_t)output_width) return 4;
    }
    return ferror(stdin) || ferror(stdout) ? 8 : 0;
}
