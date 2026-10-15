#include "library.h"
#include "sqlite3.h"

_Static_assert(_Generic(&sqlite3_malloc, void *(*)(int): 1, default: 0),
               "exact upstream SQLite allocation signature");
_Static_assert(_Generic(&sqlite3_free, void (*)(void *): 1, default: 0),
               "exact upstream SQLite release signature");

int32_t sqlite_fixture_copy_bytes(const uint8_t *bytes, size_t length,
                                  uint8_t **out_bytes, size_t *out_length) {
  if (length > 4096) return 1;
  if ((bytes == NULL && length != 0) || out_bytes == NULL || out_length == NULL) {
    return 2;
  }
  uint8_t *copy = NULL;
  if (length != 0) {
    copy = sqlite3_malloc((int)length);
    if (copy == NULL) return 1;
    for (size_t i = 0; i < length; ++i) copy[i] = bytes[i];
  }
  *out_bytes = copy;
  *out_length = length;
  return 0;
}

void sqlite_fixture_release_bytes(uint8_t *bytes) {
  sqlite3_free(bytes);
}
