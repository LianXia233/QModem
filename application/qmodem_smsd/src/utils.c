#include "utils.h"

#include <qmodem-sms/pdu.h>

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int char_to_hex(char c)
{
    if (c >= '0' && c <= '9')
        return c - '0';
    if (c >= 'a' && c <= 'f')
        return c - 'a' + 10;
    if (c >= 'A' && c <= 'F')
        return c - 'A' + 10;
    return -1;
}

int smsd_escape_json(const char *input, char *output, size_t output_size)
{
    size_t used = 0;

    if (!output || output_size == 0)
        return -1;
    if (!input)
        input = "";
    for (const char *p = input; *p; p++) {
        unsigned char c = (unsigned char)*p;
        const char *escape = NULL;
        char unicode_escape[8];
        size_t length;

        switch (*p) {
        case '"':  escape = "\\\""; break;
        case '\\': escape = "\\\\"; break;
        case '/':  escape = "\\/";  break;
        case '\b': escape = "\\b";  break;
        case '\f': escape = "\\f";  break;
        case '\n': escape = "\\n";  break;
        case '\r': escape = "\\r";  break;
        case '\t': escape = "\\t";  break;
        default:
            if (c < 0x20) {
                snprintf(unicode_escape, sizeof(unicode_escape),
                         "\\u%04x", (unsigned)c);
                escape = unicode_escape;
            }
            break;
        }
        length = escape ? strlen(escape) : 1;
        if (used + length + 1 > output_size) {
            output[used] = '\0';
            return -1;
        }
        if (escape)
            memcpy(output + used, escape, length);
        else
            output[used] = *p;
        used += length;
    }
    output[used] = '\0';
    return 0;
}

/* Append fmt to a growable heap buffer; returns 0/-1. */
struct json_buf {
    char *data;
    size_t used;
    size_t capacity;
};

static void json_buf_free(struct json_buf *jb)
{
    free(jb->data);
    jb->data = NULL;
    jb->used = jb->capacity = 0;
}

static int json_buf_appendf(struct json_buf *jb, const char *fmt, ...)
{
    va_list args;
    int written;

    if (!jb->data) {
        jb->capacity = 8192;
        jb->data = malloc(jb->capacity);
        if (!jb->data)
            return -1;
        jb->used = 0;
        jb->data[0] = '\0';
    }
    for (;;) {
        size_t remaining = jb->capacity - jb->used;
        va_start(args, fmt);
        written = vsnprintf(jb->data + jb->used, remaining, fmt, args);
        va_end(args);
        if (written < 0)
            return -1;
        if ((size_t)written < remaining)
            break;
        /* Not enough room: grow (double, under the hard cap) and retry. */
        if (jb->capacity >= SMSD_JSON_MAX_SIZE)
            return -1;
        {
            size_t capacity = jb->capacity * 2;
            char *grown;
            if (capacity > SMSD_JSON_MAX_SIZE)
                capacity = SMSD_JSON_MAX_SIZE;
            grown = realloc(jb->data, capacity);
            if (!grown)
                return -1;
            jb->data = grown;
            jb->capacity = capacity;
        }
    }
    jb->used += (size_t)written;
    return 0;
}

int smsd_display_sms_in_json(smsd_sms_t **sms, int num, char **out_json)
{
    struct json_buf jb = { NULL, 0, 0 };
    /* Worst case escape expansion is 6 bytes per input byte. */
    char *escaped_text = malloc(6 * SMSD_SMS_TEXT_SIZE + 1);

    *out_json = NULL;
    if (!escaped_text)
        return -1;
    if (json_buf_appendf(&jb, "{\"msg\":[") != 0)
        goto fail;
    for (int i = 0; i < num; i++) {
        if (!sms[i])
            continue;
        (void)smsd_escape_json(sms[i]->sms_text, escaped_text,
                               6 * SMSD_SMS_TEXT_SIZE + 1);
        if (sms[i]->ref_number) {
            if (json_buf_appendf(&jb,
                    "{\"index\":%d,\"sender\":\"%s\",\"timestamp\":%lld,"
                    "\"content\":\"%s\",\"pdu\":\"%s\","
                    "\"reference\":%d,\"total\":%d,\"part\":%d},",
                    sms[i]->sms_index, sms[i]->sender,
                    (long long)sms[i]->timestamp, escaped_text,
                    sms[i]->sms_pdu, sms[i]->ref_number,
                    sms[i]->total_segments, sms[i]->segment_number) != 0)
                goto fail;
        } else {
            if (json_buf_appendf(&jb,
                    "{\"index\":%d,\"sender\":\"%s\",\"timestamp\":%lld,"
                    "\"content\":\"%s\",\"pdu\":\"%s\"},",
                    sms[i]->sms_index, sms[i]->sender,
                    (long long)sms[i]->timestamp, escaped_text,
                    sms[i]->sms_pdu) != 0)
                goto fail;
        }
    }
    /* Drop the trailing comma of the last entry, then close the document. */
    if (jb.used > strlen("{\"msg\":[")) {
        jb.used--;
        jb.data[jb.used] = '\0';
    }
    if (json_buf_appendf(&jb, "]}") != 0)
        goto fail;
    free(escaped_text);
    *out_json = jb.data;
    return 0;
fail:
    free(escaped_text);
    json_buf_free(&jb);
    return -1;
}

int smsd_decode_pdu(smsd_sms_t *sms)
{
    char sms_text[SMSD_SMS_TEXT_SIZE] = { 0 };
    unsigned char hex_pdu[SMSD_SMS_PDU_HEX_SIZE] = { 0 };
    int tp_dcs, skip_bytes, sms_len;
    size_t pdu_str_len;

    if (!sms)
        return -1;
    pdu_str_len = strlen(sms->sms_pdu);
    if (pdu_str_len == 0 || (pdu_str_len & 1) != 0 ||
        pdu_str_len / 2 > sizeof(hex_pdu)) {
        return -1;
    }
    for (size_t i = 0; i < pdu_str_len; i += 2) {
        int high = char_to_hex(sms->sms_pdu[i]);
        int low = char_to_hex(sms->sms_pdu[i + 1]);
        if (high < 0 || low < 0)
            return -1;
        hex_pdu[i / 2] = (unsigned char)((high << 4) | low);
    }
    sms_len = pdu_decode(hex_pdu, (int)(pdu_str_len / 2),
                         &sms->timestamp,
                         sms->sender, sizeof(sms->sender),
                         sms_text, sizeof(sms_text),
                         &tp_dcs,
                         &sms->ref_number,
                         &sms->total_segments,
                         &sms->segment_number,
                         &skip_bytes);
    if (sms_len <= 0)
        return sms_len;
    sms->sms_length = sms_len;

    switch ((tp_dcs / 4) % 4) {
    case 0: {
        /* GSM 7 bit: bounded plain copy, skipping UDH padding septets. */
        size_t offset = 0;
        int i = skip_bytes > 0 ? (skip_bytes * 8 + 6) / 7 : skip_bytes;
        if (i < 0)
            i = 0;
        for (; i < sms_len && sms_text[i]; i++) {
            if (offset + 2 > sizeof(sms->sms_text))
                break;
            sms->sms_text[offset++] = sms_text[i];
        }
        sms->sms_text[offset] = '\0';
        sms->type = SMSD_CHARSET_7BIT;
        break;
    }
    case 2: {
        /*
         * UCS2: build UTF-8 directly instead of sprintf("%c") loops.
         * Bounded per code point; truncation drops whole code points so
         * multi-byte sequences are never cut in half.
         */
        size_t offset = 0;
        for (int i = skip_bytes > 0 ? skip_bytes : 0;
             i + 1 < sms_len && i + 1 < (int)sizeof(sms_text); i += 2) {
            int ucs2 = ((int)(unsigned char)sms_text[i] << 8) |
                       (unsigned char)sms_text[i + 1];
            unsigned char utf8[4];
            int length = ucs2_to_utf8(ucs2, utf8);
            if (length <= 0 || utf8[0] == '\0')
                continue; /* skip surrogates / embedded NUL units */
            if (offset + (size_t)length > sizeof(sms->sms_text) - 1)
                break; /* no room: stop, keeping sequences complete */
            memcpy(sms->sms_text + offset, utf8, (size_t)length);
            offset += (size_t)length;
        }
        sms->sms_text[offset] = '\0';
        sms->type = SMSD_CHARSET_UCS2;
        break;
    }
    default:
        break;
    }
    return sms_len;
}
