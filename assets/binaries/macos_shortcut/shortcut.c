/*
 * Shim wrapper to call ql_shortcut from the app bundle.
 * This is needed to work around macOS ARM prompting you to install Rosetta 2
 * for a simple shell script
 */

#include <mach-o/dyld.h>
#include <libgen.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>

int main(void) {
    char exe_path[PATH_MAX];
    uint32_t size = sizeof(exe_path);

    if (_NSGetExecutablePath(exe_path, &size) != 0) {
        fprintf(stderr, "Error: executable path buffer too small (needs %u bytes)\n", size);
        return 1;
    }

    printf("Executable path: %s\n", exe_path);
    char resolved[PATH_MAX];
    if (realpath(exe_path, resolved) == NULL) {
        fprintf(stderr, "Error resolving executable path: %s\n", strerror(errno));
        return 1;
    }

    char *dir = dirname(resolved);

    char script_path[PATH_MAX];
    snprintf(script_path, sizeof(script_path), "%s/ql_shortcut", dir);

    if (access(script_path, X_OK) != 0) {
        fprintf(stderr, "Error: cannot execute '%s': %s\n", script_path, strerror(errno));
        return 1;
    }

    char *args[] = {script_path, NULL};
    execv(script_path, args);
    fprintf(stderr, "Error executing '%s': %s\n", script_path, strerror(errno));

    return 1;
}
