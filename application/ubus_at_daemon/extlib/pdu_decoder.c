/*
 * pdu_decoder.c — standalone SMS PDU decoder tool (stdin -> text).
 *
 * Reads a hex string (PDU as printed by +CMGR/+CMGL without the length
 * prefix) from stdin and prints the decoded SMS-DELIVER fields.
 *
 * Hardened input path: the hex stream used to be written into a fixed
 * buffer with no bound check while iterating getchar() until '\n', so any
 * over-long stdin overflowed the buffer. Now the reader enforces a hard
 * length cap and exits with a non-zero status on overflow, invalid hex
 * digits or a dangling half-byte.
 *
 * Build: cc -Wall -Wextra -std=c99 -o pdu_decoder pdu_decoder.c
 * Usage: echo "<hex pdu>" | ./pdu_decoder
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* TS 23.040: an SMS-DELIVER TPDU is at most 176 octets; allow the SMSC
 * address header on top and a small safety margin. */
#define SMS_MAX_PDU_LENGTH 192
#define HEX_CHUNK_SIZE (2 * SMS_MAX_PDU_LENGTH)

static unsigned char pdu[SMS_MAX_PDU_LENGTH];
static size_t pdu_len;

/* Read whitespace-separated hex octets from stdin into the fixed buffer.
 * Any violation of the length cap or the hex alphabet is fatal. */
static void read_pdu_hex_stream(void)
{
    char hex[HEX_CHUNK_SIZE];
    size_t hex_len = 0;
    int c;
    int have_nibble = 0;
    unsigned char nibble = 0;

    while ((c = getchar()) != EOF) {
        if (c == '\n' || c == '\r' || c == ' ' || c == '\t')
            continue;
        if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') ||
              (c >= 'A' && c <= 'F'))) {
            fprintf(stderr, "pdu_decoder: invalid hex character 0x%02x\n",
                    (unsigned char)c);
            exit(1);
        }
        if (hex_len >= sizeof(hex)) {
            fprintf(stderr, "pdu_decoder: input exceeds %u hex digits "
                            "(%u bytes), refusing unbounded read\n",
                    (unsigned)sizeof(hex), SMS_MAX_PDU_LENGTH);
            exit(1);
        }
        hex[hex_len++] = (char)c;
    }

    if (hex_len == 0 || (hex_len % 2) != 0) {
        fprintf(stderr, "pdu_decoder: incomplete hex input "
                        "(%zu digits, need an even number)\n", hex_len);
        exit(1);
    }

    for (size_t i = 0; i < hex_len; i++) {
        char ch = hex[i];
        unsigned value;
        if (ch >= '0' && ch <= '9')
            value = (unsigned)(ch - '0');
        else if (ch >= 'a' && ch <= 'f')
            value = (unsigned)(ch - 'a' + 10);
        else
            value = (unsigned)(ch - 'A' + 10);

        if (!have_nibble) {
            nibble = (unsigned char)(value << 4);
            have_nibble = 1;
        } else {
            pdu[pdu_len++] = (unsigned char)(nibble | value);
            have_nibble = 0;
        }
    }
}

static const char gsm7_low[0x20] = {
    '@', 0, '$', 0x03, 0, 0, 0, 0, 0, 0, '\n', 0, 0, '\r', 0, 0,
    0, '_', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
};

/* Minimal GSM 03.38 default-alphabet mapping: the printable ASCII range
 * maps 1:1 except a few code points; only the common C0 subset is
 * translated explicitly (escape sequences are emitted as '?'). */
static char gsm7_to_char(unsigned char code)
{
    if (code < 0x20) {
        char mapped = gsm7_low[code];
        return mapped ? mapped : '?';
    }
    if (code == 0x40 || (code >= 0x20 && code <= 0x7f))
        return (code == 0x40) ? '@' : (char)code;
    return '?';
}

/* TP-SCTS semi-octet: low nibble is the first (tens) digit. */
static unsigned scts_field(unsigned char octet)
{
    return (unsigned)(octet & 0x0f) * 10 + (unsigned)(octet >> 4);
}

static void print_gsm7(const unsigned char *data, size_t len,
                       size_t udl_septets, size_t skip_septets)
{
    size_t total_septets = len * 8 / 7;

    if (total_septets > udl_septets)
        total_septets = udl_septets;

    for (size_t s = skip_septets; s < total_septets; s++) {
        size_t bit = s * 7;
        size_t byte = bit / 8;
        unsigned shift = (unsigned)(bit % 8);
        unsigned septet;

        septet = (unsigned)data[byte] >> shift;
        if (byte + 1 < len) {
            septet |= (unsigned)data[byte + 1] << (8 - shift);
        } else if (shift > 1) {
            break; /* final septet is only padding */
        }
        putchar(gsm7_to_char((unsigned char)(septet & 0x7fu)));
    }
}

static void print_ucs2(const unsigned char *data, size_t len)
{
    for (size_t i = 0; i + 1 < len; i += 2) {
        unsigned ucs2 = ((unsigned)data[i] << 8) | data[i + 1];
        if (ucs2 < 0x80) {
            putchar((int)ucs2);
        } else if (ucs2 < 0x800) {
            putchar((int)(0xc0 | (ucs2 >> 6)));
            putchar((int)(0x80 | (ucs2 & 0x3f)));
        } else {
            putchar((int)(0xe0 | (ucs2 >> 12)));
            putchar((int)(0x80 | ((ucs2 >> 6) & 0x3f)));
            putchar((int)(0x80 | (ucs2 & 0x3f)));
        }
    }
}

static int decode_sms_deliver(void)
{
    size_t off;
    unsigned first_octet;
    size_t addr_len;
    unsigned tp_dcs;

    if (pdu_len < 1)
        return -1;

    /* SMSC field: length octet + that many address octets (0 = absent). */
    off = 1u + (size_t)pdu[0];
    if (off >= pdu_len)
        return -1;

    first_octet = pdu[off++];
    if ((first_octet & 0x03) != 0x00) {
        fprintf(stderr, "pdu_decoder: not an SMS-DELIVER PDU "
                        "(first octet 0x%02x)\n", (unsigned char)first_octet);
        return -1;
    }

    /* Originator address: length in semi-octets, then type-of-address,
     * then the digit pairs (semi-octet swapped). */
    if (off + 2 >= pdu_len)
        return -1;
    addr_len = (size_t)pdu[off];

    printf("From:");
    for (size_t i = 0; i < addr_len; i++) {
        unsigned char pair = pdu[off + 2 + i / 2];
        putchar('0' + (int)((i % 2 == 0) ? (pair & 0x0f) : (pair >> 4)));
    }
    putchar('\n');
    off += 2 + (addr_len + 1) / 2;
    if (off + 12 >= pdu_len)
        return -1;

    /* TP-PID, TP-DCS, then TP-SCTS (7 semi-octet-swapped octets). */
    if (off + 9 > pdu_len)
        return -1;
    off++;                                   /* TP-PID */
    tp_dcs = pdu[off++];

    printf("Date/Time:20%02u%02u%02u %02u:%02u:%02u\n",
           scts_field(pdu[off]), scts_field(pdu[off + 1]),
           scts_field(pdu[off + 2]), scts_field(pdu[off + 3]),
           scts_field(pdu[off + 4]), scts_field(pdu[off + 5]));
    off += 7;

    if (off >= pdu_len)
        return -1;
    size_t udl = pdu[off++];   /* octets for UCS2, septets for GSM 7-bit */
    const unsigned char *ud = pdu + off;
    size_t avail = pdu_len - off;
    size_t skip_septets = 0;
    size_t udh_octets = 0;

    if (first_octet & 0x40) {                /* TP-UDHI: UDH present */
        if (avail < 1)
            return -1;
        udh_octets = (size_t)ud[0] + 1;
        if (udh_octets >= avail)
            return -1;
        /* Fill bits pad the UDH up to the next septet boundary. */
        skip_septets = (udh_octets * 8 + 6) / 7;
    }

    if ((tp_dcs & 0x0c) == 0x08) {
        /* UCS2: TP-UDL counts octets; user text starts after the UDH. */
        if (udl > avail)
            return -1;
        printf("Textlen=%zu\n", udl - udh_octets);
        print_ucs2(ud + udh_octets, udl - udh_octets);
    } else {
        /* GSM 7-bit: TP-UDL counts septets; octets are ceil(len*7/8).
         * Septet offsets (incl. UDH skip) stay relative to the full UD. */
        size_t need = (udl * 7 + 7) / 8;
        if (need > avail)
            return -1;
        printf("Textlen=%zu\n", udl - skip_septets);
        print_gsm7(ud, avail, udl, skip_septets);
    }
    putchar('\n');
    return 0;
}

int main(void)
{
    read_pdu_hex_stream();

    if (decode_sms_deliver() != 0) {
        fprintf(stderr, "pdu_decoder: malformed PDU (%zu bytes)\n", pdu_len);
        return 1;
    }
    return 0;
}
