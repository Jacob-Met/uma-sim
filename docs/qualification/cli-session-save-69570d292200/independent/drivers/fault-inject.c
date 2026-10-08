#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const char SENTINEL[] = "independent temporary-name collision sentinel\n";
static int collision_created = 0;
static int is_phase(const char *phase) {
    const char *value = getenv("HAMON_REVIEW_PHASE");
    return value && strcmp(value, phase) == 0;
}
static int in_review_directory(const char *name) {
    const char *directory = getenv("HAMON_REVIEW_DIR");
    if (!directory || !name) return 0;
    size_t n = strlen(directory);
    return strncmp(name, directory, n) == 0 && name[n] == '/';
}
static void absolute_name(int dirfd, const char *name, char result[PATH_MAX]) {
    result[0] = '\0';
    if (!name) return;
    if (name[0] == '/') {
        snprintf(result, PATH_MAX, "%s", name);
        return;
    }
    char base[PATH_MAX];
    if (dirfd == AT_FDCWD) {
        if (!getcwd(base, sizeof(base))) return;
    } else {
        char link[64];
        snprintf(link, sizeof(link), "/proc/self/fd/%d", dirfd);
        ssize_t n = readlink(link, base, sizeof(base)-1);
        if (n < 0) return;
        base[n] = '\0';
    }
    if (strlen(base) + strlen(name) + 2 > PATH_MAX) return;
    strcpy(result, base);
    strcat(result, "/");
    strcat(result, name);
}
static int needs_mode(int flags) {
    if (flags & O_CREAT) return 1;
#ifdef O_TMPFILE
    if ((flags & O_TMPFILE) == O_TMPFILE) return 1;
#endif
    return 0;
}
static void create_collision(int dirfd, const char *name, int flags) {
    if (!is_phase("collision") || collision_created ||
        (flags & (O_CREAT|O_EXCL)) != (O_CREAT|O_EXCL)) return;
    char absolute[PATH_MAX];
    absolute_name(dirfd, name, absolute);
    if (!in_review_directory(absolute)) return;
    collision_created = 1;
    int (*real_openat)(int,const char*,int,...) = dlsym(RTLD_NEXT, "openat");
    int fd = real_openat(dirfd, name, O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC, 0600);
    if (fd < 0) return;
    ssize_t count = write(fd, SENTINEL, sizeof(SENTINEL)-1);
    close(fd);
    if (count != (ssize_t)(sizeof(SENTINEL)-1)) return;
    dprintf(STDERR_FILENO, "HAMON_INDEPENDENT_COLLISION %s\n", absolute);
}
int open(const char *name, int flags, ...) {
    mode_t mode = 0;
    if (needs_mode(flags)) { va_list ap; va_start(ap, flags); mode = va_arg(ap, mode_t); va_end(ap); }
    int (*real_open)(const char*,int,...) = dlsym(RTLD_NEXT, "open");
    create_collision(AT_FDCWD, name, flags);
    return real_open(name, flags, mode);
}
int open64(const char *name, int flags, ...) {
    mode_t mode = 0;
    if (needs_mode(flags)) { va_list ap; va_start(ap, flags); mode = va_arg(ap, mode_t); va_end(ap); }
    int (*real_open)(const char*,int,...) = dlsym(RTLD_NEXT, "open64");
    create_collision(AT_FDCWD, name, flags);
    return real_open(name, flags, mode);
}
int openat(int dirfd, const char *name, int flags, ...) {
    mode_t mode = 0;
    if (needs_mode(flags)) { va_list ap; va_start(ap, flags); mode = va_arg(ap, mode_t); va_end(ap); }
    int (*real_openat)(int,const char*,int,...) = dlsym(RTLD_NEXT, "openat");
    create_collision(dirfd, name, flags);
    return real_openat(dirfd, name, flags, mode);
}
int openat64(int dirfd, const char *name, int flags, ...) {
    mode_t mode = 0;
    if (needs_mode(flags)) { va_list ap; va_start(ap, flags); mode = va_arg(ap, mode_t); va_end(ap); }
    int (*real_openat)(int,const char*,int,...) = dlsym(RTLD_NEXT, "openat64");
    create_collision(dirfd, name, flags);
    return real_openat(dirfd, name, flags, mode);
}
static int fail_sync(int fd, const char *call) {
    if (!is_phase("sync")) return 0;
    char link[64], name[PATH_MAX];
    snprintf(link, sizeof(link), "/proc/self/fd/%d", fd);
    ssize_t length = readlink(link, name, sizeof(name)-1);
    if (length < 0) return 0;
    name[length] = '\0';
    if (!in_review_directory(name)) return 0;
    struct stat st;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode)) return 0;
    dprintf(STDERR_FILENO, "HAMON_INDEPENDENT_FAULT sync %s bytes=%lld\n", call, (long long)st.st_size);
    errno = EIO;
    return 1;
}
int fsync(int fd) {
    if (fail_sync(fd, "fsync")) return -1;
    int (*real_fsync)(int) = dlsym(RTLD_NEXT, "fsync");
    return real_fsync(fd);
}
int fdatasync(int fd) {
    if (fail_sync(fd, "fdatasync")) return -1;
    int (*real_fdatasync)(int) = dlsym(RTLD_NEXT, "fdatasync");
    return real_fdatasync(fd);
}
int rename(const char *from, const char *to) {
    const char *target = getenv("HAMON_REVIEW_TARGET");
    if (is_phase("rename") && target && strcmp(to, target) == 0 && in_review_directory(from)) {
        struct stat st;
        long long bytes = stat(from, &st) == 0 ? (long long)st.st_size : -1;
        dprintf(STDERR_FILENO, "HAMON_INDEPENDENT_FAULT rename bytes=%lld\n", bytes);
        errno = EIO;
        return -1;
    }
    int (*real_rename)(const char*,const char*) = dlsym(RTLD_NEXT, "rename");
    return real_rename(from, to);
}
