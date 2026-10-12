#include "utils.h"

#ifdef USE_SEMAPHORE
void generate_semaphore_name(const char* filename, char* semaphore_name) {
    snprintf(semaphore_name, MAX_FILENAME_LEN, "%s%s", SEMAPHORE_PREFIX, filename);
    for (int i = 0; semaphore_name[i] != '\0'; i++) {
        if (semaphore_name[i] == '/') {
            semaphore_name[i] = '_';
        }
    }
}

int lock_at_port(char* filename){
    char semaphore_name[MAX_FILENAME_LEN];
    generate_semaphore_name(filename, semaphore_name);
    dbg_msg("semaphore_name: %s", semaphore_name);
    sem_t *sem = sem_open(semaphore_name, O_CREAT, 0644, 1);
    if (sem == SEM_FAILED) {
        perror("sem_open failed");
        return -1;
    }
    sem_wait(sem);
    return 0;
}

int unlock_at_port(char* filename){
    char semaphore_name[MAX_FILENAME_LEN];
    generate_semaphore_name(filename, semaphore_name);
    dbg_msg("semaphore_name: %s", semaphore_name);
    sem_t *sem = sem_open(semaphore_name, O_CREAT, 0644, 1);
    if (sem == SEM_FAILED) {
        perror("sem_open failed");
        return -1;
    }
    sem_post(sem);
    sem_close(sem);
    sem_unlink(semaphore_name);
    return 0;
}

#endif


static int char_to_hex(char c)
{
    // convert char to hex
    int is_digit, is_lower, is_upper;
    is_digit = c - '0';
    is_lower = c - 'a' + 10;
    is_upper = c - 'A' + 10;
    if (is_digit >= 0 && is_digit <= 9)
    {
        return is_digit;
    }
    else if (is_lower >= 10 && is_lower <= 15)
    {
        return is_lower;
    }
    else if (is_upper >= 10 && is_upper <= 15)
    {
        return is_upper;
    }
    else
    {
        return -1;
    }
}
int decode_pdu(SMS_T *sms)
{
    char sms_text[SMS_TEXT_SIZE] = {0};
    int tp_dcs;
    int skip_bytes;
    int pdu_str_len;
    unsigned char hex_pdu[SMS_PDU_HEX_SIZE] = {0};
    pdu_str_len = strlen(sms->sms_pdu);
    if (pdu_str_len == 0 || (pdu_str_len & 1) != 0 ||
        pdu_str_len / 2 > (int)sizeof(hex_pdu))
    {
        err_msg("Invalid PDU length");
        return -1;
    }
    for (int i = 0; i < pdu_str_len; i += 2)
    {
        int high = char_to_hex(sms->sms_pdu[i]);
        int low = char_to_hex(sms->sms_pdu[i + 1]);
        if (high < 0 || low < 0)
        {
            err_msg("Invalid hexadecimal digit in PDU");
            return -1;
        }
        hex_pdu[i / 2] = (unsigned char)((high << 4) | low);
    }
    int sms_len = pdu_decode(hex_pdu, pdu_str_len/2,
                             &sms->timestamp,
                             sms->sender, PHONE_NUMBER_SIZE,
                             sms_text, SMS_TEXT_SIZE,
                             &tp_dcs,
                             &sms->ref_number,
                             &sms->total_segments,
                             &sms->segment_number,
                             &skip_bytes);
    if (sms_len <= 0)
    {
        err_msg("Error decoding pdu");
        return sms_len;
    }
    sms->sms_lenght = sms_len;

    switch ((tp_dcs / 4) % 4)
    {
    case 0:
        { 
            // GSM 7 bit
            sms->type = SMS_CHARSET_7BIT;
            int i, offset = 0;
            i = skip_bytes;
            if (skip_bytes > 0)
                i = (skip_bytes * 8 + 6) / 7;
            for (; i < strlen(sms_text); i++)
                sms->sms_text[offset++] = sms_text[i];
            sms->sms_text[offset] = '\0';
            break;
        }
    case 2:
        { 
            // UCS2
            sms->type = SMS_CHARSET_UCS2;
            int offset = 0;
            for (int i = skip_bytes; i < sms_len; i += 2)
            {
                int ucs2_char = 0x000000FF & sms_text[i + 1];
                ucs2_char |= (0x0000FF00 & (sms_text[i] << 8));
                unsigned char utf8_char[5];
                int len = ucs2_to_utf8(ucs2_char, utf8_char);
                int j;
                for (j = 0; j < len; j++)
                {
                    sprintf(sms->sms_text + offset, "%c", utf8_char[j]);
                    if (utf8_char[j] != '\0')
                    {
                        offset++;
                    }
                    
                }
            }
            offset++;
            sprintf(sms->sms_text + offset, "%c", '\0');
            break;
        }
    default:
        break;
    }
    return sms_len;
}
int destroy_sms(SMS_T *sms)
{
    if (sms->sms_pdu != NULL)
    {
        free(sms->sms_pdu);
    }
    if (sms->sender != NULL)
    {
        free(sms->sender);
    }
    if (sms->sms_text != NULL)
    {
        free(sms->sms_text);
    }
    free(sms);
    return SUCCESS;
}
int dump_sms(SMS_T *sms)
{
    dbg_msg("SMS Index: %d", sms->sms_index);
    dbg_msg("SMS Text: %s", sms->sms_text);
    dbg_msg("SMS Sender: %s", sms->sender);
    dbg_msg("SMS Timestamp: %lld", (long long)sms->timestamp);
    dbg_msg("SMS Segment: %d/%d", sms->segment_number, sms->total_segments);
    return SUCCESS;
}
/* Table-driven option/operation matching: one entry per accepted token. */
typedef struct {
    char short_name;        /* single-char form ('\0' = none)   */
    const char *long_name;  /* long form without leading "--"   */
    int value;              /* enum OPTIONS / enum OPERATIONS   */
} name_map_t;

static const name_map_t g_option_map[] = {
    { AT_CMD_S,       AT_CMD_L,       AT_CMD },
    { TTY_DEV_S,      TTY_DEV_L,      TTY_DEV },
    { BAUD_RATE_S,    BAUD_RATE_L,    BAUD_RATE },
    { DATA_BITS_S,    DATA_BITS_L,    DATA_BITS },
    { PARITY_S,       PARITY_L,       PARITY },
    { STOP_BITS_S,    STOP_BITS_L,    STOP_BITS },
    { FLOW_CONTROL_S, FLOW_CONTROL_L, FLOW_CONTROL },
    { TIMEOUT_S,      TIMEOUT_L,      TIMEOUT },
    { OPERATION_S,    OPERATION_L,    OPERATION },
    { DEBUG_S,        DEBUG_L,        DEBUG },
    { SMS_PDU_S,      SMS_PDU_L,      SMS_PDU },
    { SMS_INDEX_S,    SMS_INDEX_L,    SMS_INDEX },
    { GREEDY_READ_S,  GREEDY_READ_L,  GREEDY_READ },
    { USE_UBUS_S,     USE_UBUS_L,     USE_UBUS },
};

static const name_map_t g_operation_map[] = {
    { AT_OP_S,             AT_OP_L,             AT_OP },
    { BINARY_AT_OP_S,      BINARY_AT_OP_L,      BINARY_AT_OP },
    { SMS_READ_OP_S,       SMS_READ_OP_L,       SMS_READ_OP },
    { SMS_SEND_OP_S,       SMS_SEND_OP_L,       SMS_SEND_OP },
    { SMS_DELETE_OP_S,     SMS_DELETE_OP_L,     SMS_DELETE_OP },
    { SMS_UNREAD_OP_S,     SMS_UNREAD_OP_L,     SMS_UNREAD_OP },
    { SMS_MARK_READ_OP_S,  SMS_MARK_READ_OP_L,  SMS_MARK_READ_OP },
#ifdef USE_SEMAPHORE
    { CLEANUP_SEMAPHORE_OP_S, CLEANUP_SEMAPHORE_OP_L, CLEANUP_SEMAPHORE_OP },
#endif
};

static int match_name_map(const name_map_t *map, size_t map_len,
                          char short_name, const char *long_name)
{
    for (size_t i = 0; i < map_len; i++)
    {
        if (short_name != '\0' && map[i].short_name == short_name)
            return map[i].value;
        if (long_name != NULL && map[i].long_name != NULL &&
            strcmp(long_name, map[i].long_name) == 0)
            return map[i].value;
    }
    return -1;
}

int match_option(char *option_name)
{
    // if start with '-' then it is an single character option
    if (option_name[0] == '-' && option_name[1] != '-')
    {
        return match_name_map(g_option_map, sizeof(g_option_map) / sizeof(g_option_map[0]),
                              option_name[1], NULL);
    }
    // if start with '--' then it is a long option
    if (option_name[0] == '-' && option_name[1] == '-')
    {
        return match_name_map(g_option_map, sizeof(g_option_map) / sizeof(g_option_map[0]),
                              '\0', option_name + 2);
    }
    return -1;
}
int match_operation(char *operation_name)
{
    int opstr_len = strlen(operation_name);
    if (opstr_len == 0)
    {
        return SUCCESS;
    }
    if (opstr_len == 1)
    {
        int ret = match_name_map(g_operation_map, sizeof(g_operation_map) / sizeof(g_operation_map[0]),
                                 operation_name[0], NULL);
        return ret >= 0 ? ret : INVALID_PARAM;
    }
    {
        int ret = match_name_map(g_operation_map, sizeof(g_operation_map) / sizeof(g_operation_map[0]),
                                 '\0', operation_name);
        return ret >= 0 ? ret : INVALID_PARAM;
    }
}
void escape_json(char *input, char *output)
{
    char *p = input;
    char *q = output;
    while (*p)
    {
        unsigned char c = (unsigned char)*p;
        
        if (*p == '"')
        {
            *q++ = '\\';
            *q++ = '"';
        }
        else if (*p == '\\')
        {
            *q++ = '\\';
            *q++ = '\\';
        }
        else if (*p == '/')
        {
            *q++ = '\\';
            *q++ = '/';
        }
        else if (*p == '\b')
        {
            *q++ = '\\';
            *q++ = 'b';
        }
        else if (*p == '\f')
        {
            *q++ = '\\';
            *q++ = 'f';
        }
        else if (*p == '\n')
        {
            *q++ = '\\';
            *q++ = 'n';
        }
        else if (*p == '\r')
        {
            *q++ = '\\';
            *q++ = 'r';
        }
        else if (*p == '\t')
        {
            *q++ = '\\';
            *q++ = 't';
        }
        else if (c < 0x20)  // Control characters (U+0000 through U+001F)
        {
            *q++ = '\\';
            *q++ = 'u';
            *q++ = '0';
            *q++ = '0';
            *q++ = (c >> 4) < 10 ? '0' + (c >> 4) : 'a' + (c >> 4) - 10;
            *q++ = (c & 0x0F) < 10 ? '0' + (c & 0x0F) : 'a' + (c & 0x0F) - 10;
        }
        else
        {
            *q++ = *p;
        }
        p++;
    }
    *q = '\0';
}
int usage(char* name)
{
    err_msg("Usage: %s [options]", name);
    err_msg("Or %s [device_path] [AT command]", name);
    err_msg("Or %s [device_path] [operation]", name);
    err_msg("Options:");
    err_msg("  -c, --at_cmd <AT command>  AT command");
    err_msg("  -d, --tty_dev <TTY device>  TTY device **REQUIRED**");
    err_msg("  -b, --baud_rate <baud rate>  Baud rate Default: 115200 Supported: 4800,9600,19200,38400,57600,115200");
    err_msg("  -B, --data_bits <data bits>  Data bits Default: 8 Supported: 5,6,7,8");
    err_msg("  -t, --timeout <timeout>  Default: 3 Timeout in seconds, if output is more than timeout, it will be ignored unless -g option is set");
    err_msg("  -o, --operation <operation>  Operation(at[a:defualt],binary_at[b], sms_read[r], sms_send[s], sms_delete[d], unread_sms[u], mark_read[m])");
    err_msg("  -D, --debug Debug mode Default: off");
    err_msg("  -p, --sms_pdu <sms pdu>  SMS PDU");
    err_msg("  -i, --sms_index <sms index>  SMS index");
    err_msg("  -g, --greedy_read Default: off, Greedy read mode, if set, each round it get new data from tty device, it will reset the timeout");
#ifdef ENABLE_UBUS_DAEMON
    err_msg("  -u, --use_ubus Default: off, Use UBUS AT daemon instead of direct serial access");
#endif
    #ifdef USE_SEMAPHORE
    err_msg("  -C, --cleanup Semaphore cleanup");
    #endif
    err_msg("Example:");
    err_msg("  %s -c ATI -d /dev/ttyUSB2 -b 115200 -B 8 -o at #advance at mode set bautrate and data bit", name);
    err_msg("  %s -c ATI -d /dev/ttyUSB2 # normal at mode", name);
    err_msg("  %s -c ATI -d /dev/ttyUSB2 -o binary_at -c 4154490D0A # means sending ATI to ttyUSB2", name);
    err_msg("  %s -d /dev/mhi_DUN -o r # read sms", name);
    #ifdef USE_SEMAPHORE
    err_msg("  %s -d /dev/mhi_DUN  -o C # force cleanup semaphore", name);
    #endif
    // Print help only; the caller decides the exit code and terminates.
    return INVALID_PARAM;
}

int str_to_hex(char *str, char *hex)
{
    int len = strlen(str)/2;
    int high,low;
    for (int i = 0; i < len; i++)
    {
        high = char_to_hex(str[i*2]);
        low = char_to_hex(str[i*2+1]);
        if (high == -1 || low == -1)
        {
            return INVALID_HEX;
        }
        hex[i] = (high << 4) | low;
        dbg_msg("hex[%d]: %x", i, hex[i]);
    }
    return SUCCESS;
}

int get_sms_index(char *cmgl_line)
{
    // Parse +CMGL: line to extract SMS index
    // Format: +CMGL: <index>,<stat>,<alpha>,<length>
    char *start_pos = strchr(cmgl_line, ':');
    if (start_pos == NULL) {
        return -1;
    }
    
    start_pos++; // Skip ':'
    while (*start_pos == ' ') start_pos++; // Skip spaces
    
    char *end_pos = strchr(start_pos, ',');
    if (end_pos == NULL) {
        return -1;
    }
    
    char index_str[16];
    int len = end_pos - start_pos;
    if (len >= sizeof(index_str)) {
        return -1;
    }
    
    strncpy(index_str, start_pos, len);
    index_str[len] = '\0';
    
    return atoi(index_str);
}

void dump_profile()
{
    dbg_msg("AT command: %s", s_profile.at_cmd);
    dbg_msg("TTY device: %s", s_profile.tty_dev);
    dbg_msg("Baud rate: %d", s_profile.baud_rate);
    dbg_msg("Data bits: %d", s_profile.data_bits);
    dbg_msg("Parity: %s", s_profile.parity);
    dbg_msg("Stop bits: %d", s_profile.stop_bits);
    dbg_msg("Flow control: %s", s_profile.flow_control);
    dbg_msg("Timeout: %d", s_profile.timeout);
    dbg_msg("Operation: %d", s_profile.op);
    dbg_msg("Debug: %d", s_profile.debug);
    dbg_msg("SMS PDU: %s", s_profile.sms_pdu);
    dbg_msg("SMS index: %d", s_profile.sms_index);
    dbg_msg("Greedy read: %d", s_profile.greedy_read);
    dbg_msg("Transport type: %s", s_profile.transport == TRANSPORT_UBUS ? "UBUS" : "TTY");
}
int display_sms_in_json(SMS_T **sms,int num)
{

    char msg_json[SMS_BUF_SIZE];
    int offset;
    offset = sprintf(msg_json, "{\"msg\":[");
    for (int i = 0; i < num; i++)
    {
        char escaped_text[SMS_TEXT_SIZE];
        escape_json(sms[i]->sms_text, escaped_text);
        if (sms[i]->ref_number)
            offset += sprintf(msg_json + offset, "{\"index\":%d,\"sender\":\"%s\",\"timestamp\":%lld,\"content\":\"%s\",\"pdu\":\"%s\",\"reference\":%d,\"total\":%d,\"part\":%d},",
                          sms[i]->sms_index, sms[i]->sender, (long long)sms[i]->timestamp, escaped_text, sms[i]->sms_pdu, sms[i]->ref_number, sms[i]->total_segments, sms[i]->segment_number);
        else
            offset += sprintf(msg_json + offset, "{\"index\":%d,\"sender\":\"%s\",\"timestamp\":%lld,\"content\":\"%s\",\"pdu\":\"%s\"},",
                          sms[i]->sms_index, sms[i]->sender, (long long)sms[i]->timestamp, escaped_text, sms[i]->sms_pdu);
    }
    
    //if not empty msg_json,remove the last ','
    if (offset > 10)
    {
        offset--;
    }
    offset += sprintf(msg_json + offset, "]}");
    user_msg("%s\n", msg_json);
    return SUCCESS;

    
}
