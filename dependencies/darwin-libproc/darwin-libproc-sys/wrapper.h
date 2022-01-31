/**
 * This file is used in combination with `bindgen` tool,
 * see `build.sh` file for details.
 */

#define __APPLE__ 1

#include "sys/proc_info.h"
#include "libproc.h"

/* rusage */
#include "sys/resource.h"
