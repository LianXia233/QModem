/*
 * qmodem-smsd: safe SMS utility helpers.
 *
 * Fixed port of the defective helpers found in tom_modem/src/utils.c
 * (audited as C4-高):
 *   - display_sms_in_json(): unbounded sprintf into a 256 KiB stack array
 *     with a 512-byte escape buffer that overflows by design.
 *   - decode_pdu() UCS2 branch: per-byte sprintf("%c") writes past the
 *     512-byte sms_text buffer and mis-manages the offset.
 * All string building here is bounds-checked and heap-backed; the caller
 * only ever sees NUL-terminated, UTF-8-safe output.
 */
#ifndef QMODEM_SMSD_UTILS_H
#define QMODEM_SMSD_UTILS_H

#include <stddef.h>
#include <time.h>

#define SMSD_SMS_TEXT_SIZE 512
#define SMSD_PHONE_NUMBER_SIZE 64
#define SMSD_SMS_PDU_HEX_SIZE 512
/* Cap for the heap-backed JSON builder (former SMS_BUF_SIZE stack array). */
#define SMSD_JSON_MAX_SIZE 262144

enum {
    SMSD_CHARSET_7BIT = 0,
    SMSD_CHARSET_UCS2 = 2
};

typedef struct smsd_sms {
    int sms_index;
    int sms_length;
    int ref_number;
    int segment_number;
    time_t timestamp;
    int total_segments;
    int type; /* SMSD_CHARSET_* */
    char sender[SMSD_PHONE_NUMBER_SIZE];
    char sms_text[SMSD_SMS_TEXT_SIZE];
    char sms_pdu[2 * SMSD_SMS_PDU_HEX_SIZE + 1];
} smsd_sms_t;

/*
 * JSON-escape input into output (always NUL-terminated).
 * Returns 0 when the full escaped text fits, -1 when truncated.
 * Worst-case expansion is 6 bytes per input byte, so a caller providing
 * 6 * strlen(input) + 1 bytes never sees truncation.
 */
int smsd_escape_json(const char *input, char *output, size_t output_size);

/*
 * Decode sms->sms_pdu into sender/sms_text/timestamp/segment fields.
 * Bounded at every write; UCS2 is converted to UTF-8 directly and is
 * truncated on whole code points only (never cuts a multi-byte sequence).
 * Returns the decoded length, or a negative value on error.
 */
int smsd_decode_pdu(smsd_sms_t *sms);

/*
 * Build the same JSON document as the legacy display_sms_in_json(), but
 * into a heap buffer that grows under the SMSD_JSON_MAX_SIZE cap.
 * On success stores a malloc'd string in *out_json (caller frees) and
 * returns 0. Returns -1 on allocation failure or cap overflow.
 */
int smsd_display_sms_in_json(smsd_sms_t **sms, int num, char **out_json);

#endif
