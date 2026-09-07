extern uint32_t zryna_m3_finish_invocation(void);
static uint32_t trap_status, trace_count, fault_code, fault_at, fault_attempt, tracing;
static uint32_t trace_words[4096];
uint32_t zryna_m3_observe(uint32_t command) {
  uint32_t mode = command & UINT32_C(0xf0000000);
  uint32_t index = command & UINT32_C(0x0fffffff);
  if ((command & UINT32_C(0x80000000)) != 0) {
    if (!tracing) return 0;
    if (trace_count >= 4096) { trace_count = 4097; return 0; }
    trace_words[trace_count++] = command & UINT32_C(0x7fffffff);
    return 0;
  }
  if (mode == UINT32_C(0x40000000)) {
    if (index == 0) return trace_count;
    if (index > trace_count || index > 4096) return UINT32_MAX;
    return trace_words[index - 1];
  }
  if (mode == UINT32_C(0x20000000)) {
    tracing = 1;
    fault_code = (command >> 24) & 15U;
    fault_at = command & UINT32_C(0x00ffffff);
    fault_attempt = 0;
    return 0;
  }
  if (mode == UINT32_C(0x10000000)) {
    if (index == fault_code && ++fault_attempt == fault_at) trap_status = index == 6 ? 2 : index;
    return trap_status;
  }
  if (command > 5) { trap_status = 255; return trap_status; }
  if (command && !trap_status) trap_status = command;
  return trap_status;
}
static int emit_word(uint32_t word) {
  unsigned char frame[4] = { (unsigned char)word, (unsigned char)(word >> 8),
    (unsigned char)(word >> 16), (unsigned char)(word >> 24) };
  return fwrite(frame, 1, 4, stdout) == 4;
}
static int configure_fault(int argc, char **argv) {
  uint32_t word = 0;
  if (argc == 1) return 1;
  if (argc != 2) return 0;
  for (const char *p = argv[1]; *p; ++p) {
    if (*p < '0' || *p > '9' || word > UINT32_MAX / 10U) return 0;
    uint32_t digit = (uint32_t)(*p - '0');
    if (word * 10U > UINT32_MAX - digit) return 0;
    word = word * 10U + digit;
  }
  if ((word & UINT32_C(0xf0000000)) != UINT32_C(0x20000000) ||
      ((word >> 24) & 15U) < 2 || ((word >> 24) & 15U) > 6 ||
      !(word & UINT32_C(0xffffff)) || (word & UINT32_C(0xffffff)) > 1048576) return 0;
  zryna_m3_observe(word);
  return 1;
}
