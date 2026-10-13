#include "ubus_at_daemon.h"

extern at_daemon_ctx_t g_daemon_ctx;

uint64_t at_monotonic_ms(void)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (uint64_t)now.tv_sec * 1000U + (uint64_t)now.tv_nsec / 1000000U;
}

static void expire_leases(uint64_t now)
{
    at_lease_t **cursor = &g_daemon_ctx.leases;

    while (*cursor) {
        if ((*cursor)->expires_ms <= now) {
            at_lease_t *expired = *cursor;
            *cursor = expired->next;
            free(expired);
        } else {
            cursor = &(*cursor)->next;
        }
    }
}

static int valid_component(const char *value, size_t maximum)
{
    size_t length;

    if (!value || !*value)
        return 0;
    length = strnlen(value, maximum);
    return length > 0 && length < maximum;
}

// Fill buf with 8 bytes of kernel entropy from /dev/urandom so lease
// tokens are not predictable (the previous realtime-clock + counter scheme
// was guessable). Returns 0 on success, -1 if entropy is unavailable.
static int read_random_token_bytes(unsigned char *buf, size_t len)
{
    int fd = open("/dev/urandom", O_RDONLY);
    size_t got = 0;

    if (fd < 0)
        return -1;
    while (got < len) {
        ssize_t n = read(fd, buf + got, len - got);
        if (n <= 0) {
            close(fd);
            return -1;
        }
        got += (size_t)n;
    }
    close(fd);
    return 0;
}

int at_lease_acquire(const char *port, const char *owner, unsigned int ttl,
                     char *token, size_t token_size, uint64_t *expires_ms)
{
    at_lease_t *lease;
    uint64_t now = at_monotonic_ms();

    if (!valid_component(port, MAX_PORT_PATH_SIZE) ||
        !valid_component(owner, sizeof(lease->owner)) || ttl < 1 || ttl > 300)
        return -1;

    pthread_mutex_lock(&g_daemon_ctx.control_mutex);
    expire_leases(now);
    for (lease = g_daemon_ctx.leases; lease; lease = lease->next) {
        if (!strcmp(lease->port, port)) {
            pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
            return -2;
        }
    }
    lease = calloc(1, sizeof(*lease));
    if (!lease) {
        pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
        return -3;
    }
    snprintf(lease->port, sizeof(lease->port), "%s", port);
    snprintf(lease->owner, sizeof(lease->owner), "%s", owner);
    {
        // Unpredictable token: 8 bytes from /dev/urandom (fall back to the
        // daemon epoch if unavailable), disambiguated by a monotonically
        // increasing counter which also guarantees uniqueness.
        unsigned char rnd[8];
        uint64_t random_part = g_daemon_ctx.daemon_epoch;

        if (read_random_token_bytes(rnd, sizeof(rnd)) == 0) {
            for (size_t i = 0; i < sizeof(rnd); i++)
                random_part = (random_part << 8) | rnd[i];
        }
        snprintf(lease->token, sizeof(lease->token), "%llx-%llx",
                 (unsigned long long)random_part,
                 (unsigned long long)++g_daemon_ctx.next_lease_id);
    }
    lease->expires_ms = now + (uint64_t)ttl * 1000U;
    lease->next = g_daemon_ctx.leases;
    g_daemon_ctx.leases = lease;
    snprintf(token, token_size, "%s", lease->token);
    *expires_ms = lease->expires_ms;
    pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
    return 0;
}

int at_lease_renew(const char *port, const char *owner, const char *token,
                   unsigned int ttl, uint64_t *expires_ms)
{
    at_lease_t *lease;
    uint64_t now = at_monotonic_ms();

    if (!valid_component(port, MAX_PORT_PATH_SIZE) ||
        !valid_component(owner, sizeof(lease->owner)) ||
        !valid_component(token, sizeof(lease->token)) || ttl < 1 || ttl > 300)
        return -1;
    pthread_mutex_lock(&g_daemon_ctx.control_mutex);
    expire_leases(now);
    for (lease = g_daemon_ctx.leases; lease; lease = lease->next) {
        if (!strcmp(lease->port, port) && !strcmp(lease->owner, owner) &&
            !strcmp(lease->token, token)) {
            lease->expires_ms = now + (uint64_t)ttl * 1000U;
            *expires_ms = lease->expires_ms;
            pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
            return 0;
        }
    }
    pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
    return -2;
}

int at_lease_release(const char *port, const char *owner, const char *token)
{
    at_lease_t **cursor;

    pthread_mutex_lock(&g_daemon_ctx.control_mutex);
    expire_leases(at_monotonic_ms());
    cursor = &g_daemon_ctx.leases;
    while (*cursor) {
        at_lease_t *lease = *cursor;
        if (!strcmp(lease->port, port) && !strcmp(lease->owner, owner) &&
            !strcmp(lease->token, token)) {
            *cursor = lease->next;
            free(lease);
            pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
            return 0;
        }
        cursor = &lease->next;
    }
    pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
    return -1;
}

int at_lease_authorize(const char *port, const char *owner)
{
    at_lease_t *lease;
    int result = 0;

    pthread_mutex_lock(&g_daemon_ctx.control_mutex);
    expire_leases(at_monotonic_ms());
    for (lease = g_daemon_ctx.leases; lease; lease = lease->next) {
        if (!strcmp(lease->port, port)) {
            result = owner && !strcmp(lease->owner, owner) ? 0 : -1;
            break;
        }
    }
    pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
    return result;
}

int at_urc_register(const char *port, const char *owner, const char *urc_id,
                    const char *prefix)
{
    at_urc_registration_t *urc;
    at_urc_registration_t **cursor;

    if (!valid_component(port, MAX_PORT_PATH_SIZE) ||
        !valid_component(owner, sizeof(urc->owner)) ||
        !valid_component(urc_id, sizeof(urc->urc_id)) ||
        !valid_component(prefix, sizeof(urc->prefix)))
        return -1;
    pthread_mutex_lock(&g_daemon_ctx.control_mutex);
    cursor = &g_daemon_ctx.urcs;
    while (*cursor) {
        urc = *cursor;
        if (!strcmp(urc->owner, owner) && !strcmp(urc->urc_id, urc_id)) {
            if (strcmp(urc->port, port)) {
                *cursor = urc->next;
                free(urc);
                continue;
            }
            snprintf(urc->prefix, sizeof(urc->prefix), "%s", prefix);
            pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
            return 0;
        }
        cursor = &urc->next;
    }
    urc = calloc(1, sizeof(*urc));
    if (!urc) {
        pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
        return -2;
    }
    snprintf(urc->port, sizeof(urc->port), "%s", port);
    snprintf(urc->owner, sizeof(urc->owner), "%s", owner);
    snprintf(urc->urc_id, sizeof(urc->urc_id), "%s", urc_id);
    snprintf(urc->prefix, sizeof(urc->prefix), "%s", prefix);
    urc->next = g_daemon_ctx.urcs;
    g_daemon_ctx.urcs = urc;
    pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
    return 0;
}

int at_urc_unregister(const char *port, const char *owner, const char *urc_id)
{
    at_urc_registration_t **cursor;

    pthread_mutex_lock(&g_daemon_ctx.control_mutex);
    cursor = &g_daemon_ctx.urcs;
    while (*cursor) {
        at_urc_registration_t *urc = *cursor;
        if (!strcmp(urc->port, port) && !strcmp(urc->owner, owner) &&
            !strcmp(urc->urc_id, urc_id)) {
            *cursor = urc->next;
            free(urc);
            pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
            return 0;
        }
        cursor = &urc->next;
    }
    pthread_mutex_unlock(&g_daemon_ctx.control_mutex);
    return -1;
}

void at_control_cleanup(void)
{
    at_lease_t *lease;
    at_urc_registration_t *urc;

    while ((lease = g_daemon_ctx.leases) != NULL) {
        g_daemon_ctx.leases = lease->next;
        free(lease);
    }
    while ((urc = g_daemon_ctx.urcs) != NULL) {
        g_daemon_ctx.urcs = urc->next;
        free(urc);
    }
}
