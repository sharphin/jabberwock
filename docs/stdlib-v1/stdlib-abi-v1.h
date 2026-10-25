/* Generated design declarations only; no runtime implementation. */
#ifndef JW_STDLIB_ABI_V1_H
#define JW_STDLIB_ABI_V1_H
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
#define JW_SA static_assert
#define JW_ALIGN alignof
extern "C" {
#else
#define JW_SA _Static_assert
#define JW_ALIGN _Alignof
#endif
typedef struct { uint64_t slot, generation; uint32_t kind, access; } JwResTokenV1;
typedef struct { const uint8_t *data; uint64_t length; } JwSliceV1;
typedef struct { uint8_t *data; uint64_t length, capacity; } JwBufferV1;
typedef struct { uint32_t valid, effect; uint64_t progress; uint32_t outcome, reserved; JwBufferV1 value; } JwOutcomeV1;
JW_SA(sizeof(void *) == 8, "64-bit ABI required");
JW_SA(sizeof(JwResTokenV1) == 24 && JW_ALIGN(JwResTokenV1) == 8, "token layout");
JW_SA(offsetof(JwResTokenV1, generation) == 8 && offsetof(JwResTokenV1, kind) == 16 && offsetof(JwResTokenV1, access) == 20, "token offsets");
JW_SA(sizeof(JwSliceV1) == 16 && sizeof(JwBufferV1) == 24, "buffer layout");
JW_SA(sizeof(JwOutcomeV1) == 48 && offsetof(JwOutcomeV1, value) == 24 && offsetof(JwOutcomeV1, outcome) == 16, "outcome layout");
int32_t jw_res_require_abi_v1(uint32_t abi, const uint8_t *registry_sha256_32);
int32_t jw_res_context_new_v1(uint32_t abi, void **out_context);
int32_t jw_res_context_finish_v1(void *context);
int32_t jw_res_close_v1(void *context, JwResTokenV1 *inout_owner);
int32_t jw_res_drop_v1(void *context, JwResTokenV1 *inout_owner);
int32_t jw_buffer_release_v1(void *context, JwBufferV1 *inout_buffer);
/* std.fs.Path.from_text */
typedef struct { JwSliceV1 text; } JwArgs_00010001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010001V1;
int32_t jw_fs_path_from_text_v1(void *context, JwArgs_00010001V1 *inout_args, JwOut_00010001V1 *out);
JW_SA(sizeof(JwArgs_00010001V1) == 16 && JW_ALIGN(JwArgs_00010001V1) == 8, "args 00010001");
JW_SA(offsetof(JwArgs_00010001V1, text) == 0, "field 00010001.text");
JW_SA(sizeof(JwOut_00010001V1) == 48, "out 00010001");
/* std.fs.Path.to_text */
typedef struct { JwSliceV1 self; } JwArgs_00010002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010002V1;
int32_t jw_fs_path_to_text_v1(void *context, JwArgs_00010002V1 *inout_args, JwOut_00010002V1 *out);
JW_SA(sizeof(JwArgs_00010002V1) == 16 && JW_ALIGN(JwArgs_00010002V1) == 8, "args 00010002");
JW_SA(offsetof(JwArgs_00010002V1, self) == 0, "field 00010002.self");
JW_SA(sizeof(JwOut_00010002V1) == 48, "out 00010002");
/* std.fs.Path.display */
typedef struct { JwSliceV1 self; } JwArgs_00010003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010003V1;
int32_t jw_fs_path_display_v1(void *context, JwArgs_00010003V1 *inout_args, JwOut_00010003V1 *out);
JW_SA(sizeof(JwArgs_00010003V1) == 16 && JW_ALIGN(JwArgs_00010003V1) == 8, "args 00010003");
JW_SA(offsetof(JwArgs_00010003V1, self) == 0, "field 00010003.self");
JW_SA(sizeof(JwOut_00010003V1) == 48, "out 00010003");
/* std.fs.Path.file_name */
typedef struct { JwSliceV1 self; } JwArgs_00010004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010004V1;
int32_t jw_fs_path_file_name_v1(void *context, JwArgs_00010004V1 *inout_args, JwOut_00010004V1 *out);
JW_SA(sizeof(JwArgs_00010004V1) == 16 && JW_ALIGN(JwArgs_00010004V1) == 8, "args 00010004");
JW_SA(offsetof(JwArgs_00010004V1, self) == 0, "field 00010004.self");
JW_SA(sizeof(JwOut_00010004V1) == 48, "out 00010004");
/* std.fs.Path.join */
typedef struct { JwSliceV1 self; JwSliceV1 child; } JwArgs_00010005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010005V1;
int32_t jw_fs_path_join_v1(void *context, JwArgs_00010005V1 *inout_args, JwOut_00010005V1 *out);
JW_SA(sizeof(JwArgs_00010005V1) == 32 && JW_ALIGN(JwArgs_00010005V1) == 8, "args 00010005");
JW_SA(offsetof(JwArgs_00010005V1, self) == 0, "field 00010005.self");
JW_SA(offsetof(JwArgs_00010005V1, child) == 16, "field 00010005.child");
JW_SA(sizeof(JwOut_00010005V1) == 48, "out 00010005");
/* std.fs.Path.absolute */
typedef struct { JwSliceV1 self; } JwArgs_00010006V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010006V1;
int32_t jw_fs_path_absolute_v1(void *context, JwArgs_00010006V1 *inout_args, JwOut_00010006V1 *out);
JW_SA(sizeof(JwArgs_00010006V1) == 16 && JW_ALIGN(JwArgs_00010006V1) == 8, "args 00010006");
JW_SA(offsetof(JwArgs_00010006V1, self) == 0, "field 00010006.self");
JW_SA(sizeof(JwOut_00010006V1) == 48, "out 00010006");
/* std.fs.Path.canonical */
typedef struct { JwSliceV1 self; } JwArgs_00010007V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010007V1;
int32_t jw_fs_path_canonical_v1(void *context, JwArgs_00010007V1 *inout_args, JwOut_00010007V1 *out);
JW_SA(sizeof(JwArgs_00010007V1) == 16 && JW_ALIGN(JwArgs_00010007V1) == 8, "args 00010007");
JW_SA(offsetof(JwArgs_00010007V1, self) == 0, "field 00010007.self");
JW_SA(sizeof(JwOut_00010007V1) == 48, "out 00010007");
/* std.fs.WindowsShare.defaults */
typedef struct { uint64_t reserved; } JwArgs_00010008V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010008V1;
int32_t jw_fs_windows_share_defaults_v1(void *context, JwArgs_00010008V1 *inout_args, JwOut_00010008V1 *out);
JW_SA(sizeof(JwArgs_00010008V1) == 8 && JW_ALIGN(JwArgs_00010008V1) == 8, "args 00010008");
JW_SA(sizeof(JwOut_00010008V1) == 48, "out 00010008");
/* std.fs.OpenOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00010009V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010009V1;
int32_t jw_fs_open_options_defaults_v1(void *context, JwArgs_00010009V1 *inout_args, JwOut_00010009V1 *out);
JW_SA(sizeof(JwArgs_00010009V1) == 8 && JW_ALIGN(JwArgs_00010009V1) == 8, "args 00010009");
JW_SA(sizeof(JwOut_00010009V1) == 48, "out 00010009");
/* std.fs.open */
typedef struct { JwSliceV1 path; JwSliceV1 options; } JwArgs_0001000aV1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_0001000aV1;
int32_t jw_fs_open_v1(void *context, JwArgs_0001000aV1 *inout_args, JwOut_0001000aV1 *out);
JW_SA(sizeof(JwArgs_0001000aV1) == 32 && JW_ALIGN(JwArgs_0001000aV1) == 8, "args 0001000a");
JW_SA(offsetof(JwArgs_0001000aV1, path) == 0, "field 0001000a.path");
JW_SA(offsetof(JwArgs_0001000aV1, options) == 16, "field 0001000a.options");
JW_SA(sizeof(JwOut_0001000aV1) == 72, "out 0001000a");
/* std.fs.read_bytes */
typedef struct { JwSliceV1 path; uint64_t limit; } JwArgs_0001000bV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001000bV1;
int32_t jw_fs_read_bytes_v1(void *context, JwArgs_0001000bV1 *inout_args, JwOut_0001000bV1 *out);
JW_SA(sizeof(JwArgs_0001000bV1) == 24 && JW_ALIGN(JwArgs_0001000bV1) == 8, "args 0001000b");
JW_SA(offsetof(JwArgs_0001000bV1, path) == 0, "field 0001000b.path");
JW_SA(offsetof(JwArgs_0001000bV1, limit) == 16, "field 0001000b.limit");
JW_SA(sizeof(JwOut_0001000bV1) == 48, "out 0001000b");
/* std.fs.read_text */
typedef struct { JwSliceV1 path; uint64_t limit; } JwArgs_0001000cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001000cV1;
int32_t jw_fs_read_text_v1(void *context, JwArgs_0001000cV1 *inout_args, JwOut_0001000cV1 *out);
JW_SA(sizeof(JwArgs_0001000cV1) == 24 && JW_ALIGN(JwArgs_0001000cV1) == 8, "args 0001000c");
JW_SA(offsetof(JwArgs_0001000cV1, path) == 0, "field 0001000c.path");
JW_SA(offsetof(JwArgs_0001000cV1, limit) == 16, "field 0001000c.limit");
JW_SA(sizeof(JwOut_0001000cV1) == 48, "out 0001000c");
/* std.fs.read_dir */
typedef struct { JwSliceV1 path; uint64_t limit; } JwArgs_0001000dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001000dV1;
int32_t jw_fs_read_dir_v1(void *context, JwArgs_0001000dV1 *inout_args, JwOut_0001000dV1 *out);
JW_SA(sizeof(JwArgs_0001000dV1) == 24 && JW_ALIGN(JwArgs_0001000dV1) == 8, "args 0001000d");
JW_SA(offsetof(JwArgs_0001000dV1, path) == 0, "field 0001000d.path");
JW_SA(offsetof(JwArgs_0001000dV1, limit) == 16, "field 0001000d.limit");
JW_SA(sizeof(JwOut_0001000dV1) == 48, "out 0001000d");
/* std.fs.write_bytes */
typedef struct { JwSliceV1 path; JwSliceV1 data; JwSliceV1 mode; } JwArgs_0001000eV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001000eV1;
int32_t jw_fs_write_bytes_v1(void *context, JwArgs_0001000eV1 *inout_args, JwOut_0001000eV1 *out);
JW_SA(sizeof(JwArgs_0001000eV1) == 48 && JW_ALIGN(JwArgs_0001000eV1) == 8, "args 0001000e");
JW_SA(offsetof(JwArgs_0001000eV1, path) == 0, "field 0001000e.path");
JW_SA(offsetof(JwArgs_0001000eV1, data) == 16, "field 0001000e.data");
JW_SA(offsetof(JwArgs_0001000eV1, mode) == 32, "field 0001000e.mode");
JW_SA(sizeof(JwOut_0001000eV1) == 48, "out 0001000e");
/* std.fs.metadata */
typedef struct { JwSliceV1 path; JwSliceV1 follow; } JwArgs_0001000fV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001000fV1;
int32_t jw_fs_metadata_v1(void *context, JwArgs_0001000fV1 *inout_args, JwOut_0001000fV1 *out);
JW_SA(sizeof(JwArgs_0001000fV1) == 32 && JW_ALIGN(JwArgs_0001000fV1) == 8, "args 0001000f");
JW_SA(offsetof(JwArgs_0001000fV1, path) == 0, "field 0001000f.path");
JW_SA(offsetof(JwArgs_0001000fV1, follow) == 16, "field 0001000f.follow");
JW_SA(sizeof(JwOut_0001000fV1) == 48, "out 0001000f");
/* std.fs.create_dir */
typedef struct { JwSliceV1 path; } JwArgs_00010010V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010010V1;
int32_t jw_fs_create_dir_v1(void *context, JwArgs_00010010V1 *inout_args, JwOut_00010010V1 *out);
JW_SA(sizeof(JwArgs_00010010V1) == 16 && JW_ALIGN(JwArgs_00010010V1) == 8, "args 00010010");
JW_SA(offsetof(JwArgs_00010010V1, path) == 0, "field 00010010.path");
JW_SA(sizeof(JwOut_00010010V1) == 48, "out 00010010");
/* std.fs.remove_file */
typedef struct { JwSliceV1 path; } JwArgs_00010011V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010011V1;
int32_t jw_fs_remove_file_v1(void *context, JwArgs_00010011V1 *inout_args, JwOut_00010011V1 *out);
JW_SA(sizeof(JwArgs_00010011V1) == 16 && JW_ALIGN(JwArgs_00010011V1) == 8, "args 00010011");
JW_SA(offsetof(JwArgs_00010011V1, path) == 0, "field 00010011.path");
JW_SA(sizeof(JwOut_00010011V1) == 48, "out 00010011");
/* std.fs.remove_empty_dir */
typedef struct { JwSliceV1 path; } JwArgs_00010012V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010012V1;
int32_t jw_fs_remove_empty_dir_v1(void *context, JwArgs_00010012V1 *inout_args, JwOut_00010012V1 *out);
JW_SA(sizeof(JwArgs_00010012V1) == 16 && JW_ALIGN(JwArgs_00010012V1) == 8, "args 00010012");
JW_SA(offsetof(JwArgs_00010012V1, path) == 0, "field 00010012.path");
JW_SA(sizeof(JwOut_00010012V1) == 48, "out 00010012");
/* std.fs.rename */
typedef struct { JwSliceV1 source; JwSliceV1 target; JwSliceV1 replace; } JwArgs_00010013V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010013V1;
int32_t jw_fs_rename_v1(void *context, JwArgs_00010013V1 *inout_args, JwOut_00010013V1 *out);
JW_SA(sizeof(JwArgs_00010013V1) == 48 && JW_ALIGN(JwArgs_00010013V1) == 8, "args 00010013");
JW_SA(offsetof(JwArgs_00010013V1, source) == 0, "field 00010013.source");
JW_SA(offsetof(JwArgs_00010013V1, target) == 16, "field 00010013.target");
JW_SA(offsetof(JwArgs_00010013V1, replace) == 32, "field 00010013.replace");
JW_SA(sizeof(JwOut_00010013V1) == 48, "out 00010013");
/* std.fs.File.read */
typedef struct { JwResTokenV1 self; uint64_t max_bytes; } JwArgs_00010014V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010014V1;
int32_t jw_fs_file_read_v1(void *context, JwArgs_00010014V1 *inout_args, JwOut_00010014V1 *out);
JW_SA(sizeof(JwArgs_00010014V1) == 32 && JW_ALIGN(JwArgs_00010014V1) == 8, "args 00010014");
JW_SA(offsetof(JwArgs_00010014V1, self) == 0, "field 00010014.self");
JW_SA(offsetof(JwArgs_00010014V1, max_bytes) == 24, "field 00010014.max_bytes");
JW_SA(sizeof(JwOut_00010014V1) == 48, "out 00010014");
/* std.fs.File.write */
typedef struct { JwResTokenV1 self; JwSliceV1 data; } JwArgs_00010015V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010015V1;
int32_t jw_fs_file_write_v1(void *context, JwArgs_00010015V1 *inout_args, JwOut_00010015V1 *out);
JW_SA(sizeof(JwArgs_00010015V1) == 40 && JW_ALIGN(JwArgs_00010015V1) == 8, "args 00010015");
JW_SA(offsetof(JwArgs_00010015V1, self) == 0, "field 00010015.self");
JW_SA(offsetof(JwArgs_00010015V1, data) == 24, "field 00010015.data");
JW_SA(sizeof(JwOut_00010015V1) == 48, "out 00010015");
/* std.fs.File.write_all */
typedef struct { JwResTokenV1 self; JwSliceV1 data; } JwArgs_00010016V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010016V1;
int32_t jw_fs_file_write_all_v1(void *context, JwArgs_00010016V1 *inout_args, JwOut_00010016V1 *out);
JW_SA(sizeof(JwArgs_00010016V1) == 40 && JW_ALIGN(JwArgs_00010016V1) == 8, "args 00010016");
JW_SA(offsetof(JwArgs_00010016V1, self) == 0, "field 00010016.self");
JW_SA(offsetof(JwArgs_00010016V1, data) == 24, "field 00010016.data");
JW_SA(sizeof(JwOut_00010016V1) == 48, "out 00010016");
/* std.fs.File.seek */
typedef struct { JwResTokenV1 self; int64_t offset; JwSliceV1 origin; } JwArgs_00010017V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010017V1;
int32_t jw_fs_file_seek_v1(void *context, JwArgs_00010017V1 *inout_args, JwOut_00010017V1 *out);
JW_SA(sizeof(JwArgs_00010017V1) == 48 && JW_ALIGN(JwArgs_00010017V1) == 8, "args 00010017");
JW_SA(offsetof(JwArgs_00010017V1, self) == 0, "field 00010017.self");
JW_SA(offsetof(JwArgs_00010017V1, offset) == 24, "field 00010017.offset");
JW_SA(offsetof(JwArgs_00010017V1, origin) == 32, "field 00010017.origin");
JW_SA(sizeof(JwOut_00010017V1) == 48, "out 00010017");
/* std.fs.File.length */
typedef struct { JwResTokenV1 self; } JwArgs_00010018V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010018V1;
int32_t jw_fs_file_length_v1(void *context, JwArgs_00010018V1 *inout_args, JwOut_00010018V1 *out);
JW_SA(sizeof(JwArgs_00010018V1) == 24 && JW_ALIGN(JwArgs_00010018V1) == 8, "args 00010018");
JW_SA(offsetof(JwArgs_00010018V1, self) == 0, "field 00010018.self");
JW_SA(sizeof(JwOut_00010018V1) == 48, "out 00010018");
/* std.fs.File.metadata */
typedef struct { JwResTokenV1 self; } JwArgs_00010019V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00010019V1;
int32_t jw_fs_file_metadata_v1(void *context, JwArgs_00010019V1 *inout_args, JwOut_00010019V1 *out);
JW_SA(sizeof(JwArgs_00010019V1) == 24 && JW_ALIGN(JwArgs_00010019V1) == 8, "args 00010019");
JW_SA(offsetof(JwArgs_00010019V1, self) == 0, "field 00010019.self");
JW_SA(sizeof(JwOut_00010019V1) == 48, "out 00010019");
/* std.fs.File.set_length */
typedef struct { JwResTokenV1 self; uint64_t length; } JwArgs_0001001aV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001001aV1;
int32_t jw_fs_file_set_length_v1(void *context, JwArgs_0001001aV1 *inout_args, JwOut_0001001aV1 *out);
JW_SA(sizeof(JwArgs_0001001aV1) == 32 && JW_ALIGN(JwArgs_0001001aV1) == 8, "args 0001001a");
JW_SA(offsetof(JwArgs_0001001aV1, self) == 0, "field 0001001a.self");
JW_SA(offsetof(JwArgs_0001001aV1, length) == 24, "field 0001001a.length");
JW_SA(sizeof(JwOut_0001001aV1) == 48, "out 0001001a");
/* std.fs.File.flush */
typedef struct { JwResTokenV1 self; } JwArgs_0001001bV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001001bV1;
int32_t jw_fs_file_flush_v1(void *context, JwArgs_0001001bV1 *inout_args, JwOut_0001001bV1 *out);
JW_SA(sizeof(JwArgs_0001001bV1) == 24 && JW_ALIGN(JwArgs_0001001bV1) == 8, "args 0001001b");
JW_SA(offsetof(JwArgs_0001001bV1, self) == 0, "field 0001001b.self");
JW_SA(sizeof(JwOut_0001001bV1) == 48, "out 0001001b");
/* std.fs.File.sync */
typedef struct { JwResTokenV1 self; } JwArgs_0001001cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001001cV1;
int32_t jw_fs_file_sync_v1(void *context, JwArgs_0001001cV1 *inout_args, JwOut_0001001cV1 *out);
JW_SA(sizeof(JwArgs_0001001cV1) == 24 && JW_ALIGN(JwArgs_0001001cV1) == 8, "args 0001001c");
JW_SA(offsetof(JwArgs_0001001cV1, self) == 0, "field 0001001c.self");
JW_SA(sizeof(JwOut_0001001cV1) == 48, "out 0001001c");
/* std.fs.File.close */
typedef struct { JwResTokenV1 self; } JwArgs_0001001dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0001001dV1;
int32_t jw_fs_file_close_v1(void *context, JwArgs_0001001dV1 *inout_args, JwOut_0001001dV1 *out);
JW_SA(sizeof(JwArgs_0001001dV1) == 24 && JW_ALIGN(JwArgs_0001001dV1) == 8, "args 0001001d");
JW_SA(offsetof(JwArgs_0001001dV1, self) == 0, "field 0001001d.self");
JW_SA(sizeof(JwOut_0001001dV1) == 48, "out 0001001d");
/* std.time.now */
typedef struct { uint64_t reserved; } JwArgs_00020001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020001V1;
int32_t jw_time_now_v1(void *context, JwArgs_00020001V1 *inout_args, JwOut_00020001V1 *out);
JW_SA(sizeof(JwArgs_00020001V1) == 8 && JW_ALIGN(JwArgs_00020001V1) == 8, "args 00020001");
JW_SA(sizeof(JwOut_00020001V1) == 48, "out 00020001");
/* std.time.elapsed */
typedef struct { JwSliceV1 since; } JwArgs_00020002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020002V1;
int32_t jw_time_elapsed_v1(void *context, JwArgs_00020002V1 *inout_args, JwOut_00020002V1 *out);
JW_SA(sizeof(JwArgs_00020002V1) == 16 && JW_ALIGN(JwArgs_00020002V1) == 8, "args 00020002");
JW_SA(offsetof(JwArgs_00020002V1, since) == 0, "field 00020002.since");
JW_SA(sizeof(JwOut_00020002V1) == 48, "out 00020002");
/* std.time.Duration.from_millis */
typedef struct { uint64_t value; } JwArgs_00020003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020003V1;
int32_t jw_time_duration_from_millis_v1(void *context, JwArgs_00020003V1 *inout_args, JwOut_00020003V1 *out);
JW_SA(sizeof(JwArgs_00020003V1) == 8 && JW_ALIGN(JwArgs_00020003V1) == 8, "args 00020003");
JW_SA(offsetof(JwArgs_00020003V1, value) == 0, "field 00020003.value");
JW_SA(sizeof(JwOut_00020003V1) == 48, "out 00020003");
/* std.time.Duration.from_seconds */
typedef struct { uint64_t value; } JwArgs_00020004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020004V1;
int32_t jw_time_duration_from_seconds_v1(void *context, JwArgs_00020004V1 *inout_args, JwOut_00020004V1 *out);
JW_SA(sizeof(JwArgs_00020004V1) == 8 && JW_ALIGN(JwArgs_00020004V1) == 8, "args 00020004");
JW_SA(offsetof(JwArgs_00020004V1, value) == 0, "field 00020004.value");
JW_SA(sizeof(JwOut_00020004V1) == 48, "out 00020004");
/* std.time.Duration.from_nanos */
typedef struct { uint64_t value; } JwArgs_00020005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020005V1;
int32_t jw_time_duration_from_nanos_v1(void *context, JwArgs_00020005V1 *inout_args, JwOut_00020005V1 *out);
JW_SA(sizeof(JwArgs_00020005V1) == 8 && JW_ALIGN(JwArgs_00020005V1) == 8, "args 00020005");
JW_SA(offsetof(JwArgs_00020005V1, value) == 0, "field 00020005.value");
JW_SA(sizeof(JwOut_00020005V1) == 48, "out 00020005");
/* std.time.Duration.nanos */
typedef struct { JwSliceV1 self; } JwArgs_00020006V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020006V1;
int32_t jw_time_duration_nanos_v1(void *context, JwArgs_00020006V1 *inout_args, JwOut_00020006V1 *out);
JW_SA(sizeof(JwArgs_00020006V1) == 16 && JW_ALIGN(JwArgs_00020006V1) == 8, "args 00020006");
JW_SA(offsetof(JwArgs_00020006V1, self) == 0, "field 00020006.self");
JW_SA(sizeof(JwOut_00020006V1) == 48, "out 00020006");
/* std.time.Deadline.infinite */
typedef struct { uint64_t reserved; } JwArgs_00020007V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020007V1;
int32_t jw_time_deadline_infinite_v1(void *context, JwArgs_00020007V1 *inout_args, JwOut_00020007V1 *out);
JW_SA(sizeof(JwArgs_00020007V1) == 8 && JW_ALIGN(JwArgs_00020007V1) == 8, "args 00020007");
JW_SA(sizeof(JwOut_00020007V1) == 48, "out 00020007");
/* std.time.deadline_after */
typedef struct { JwSliceV1 duration; } JwArgs_00020008V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020008V1;
int32_t jw_time_deadline_after_v1(void *context, JwArgs_00020008V1 *inout_args, JwOut_00020008V1 *out);
JW_SA(sizeof(JwArgs_00020008V1) == 16 && JW_ALIGN(JwArgs_00020008V1) == 8, "args 00020008");
JW_SA(offsetof(JwArgs_00020008V1, duration) == 0, "field 00020008.duration");
JW_SA(sizeof(JwOut_00020008V1) == 48, "out 00020008");
/* std.time.deadline_at */
typedef struct { JwSliceV1 instant; } JwArgs_00020009V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00020009V1;
int32_t jw_time_deadline_at_v1(void *context, JwArgs_00020009V1 *inout_args, JwOut_00020009V1 *out);
JW_SA(sizeof(JwArgs_00020009V1) == 16 && JW_ALIGN(JwArgs_00020009V1) == 8, "args 00020009");
JW_SA(offsetof(JwArgs_00020009V1, instant) == 0, "field 00020009.instant");
JW_SA(sizeof(JwOut_00020009V1) == 48, "out 00020009");
/* std.net.IpAddress.v4 */
typedef struct { JwSliceV1 bytes; } JwArgs_00030001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030001V1;
int32_t jw_net_ip_address_v4_v1(void *context, JwArgs_00030001V1 *inout_args, JwOut_00030001V1 *out);
JW_SA(sizeof(JwArgs_00030001V1) == 16 && JW_ALIGN(JwArgs_00030001V1) == 8, "args 00030001");
JW_SA(offsetof(JwArgs_00030001V1, bytes) == 0, "field 00030001.bytes");
JW_SA(sizeof(JwOut_00030001V1) == 48, "out 00030001");
/* std.net.IpAddress.v6 */
typedef struct { JwSliceV1 bytes; } JwArgs_00030002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030002V1;
int32_t jw_net_ip_address_v6_v1(void *context, JwArgs_00030002V1 *inout_args, JwOut_00030002V1 *out);
JW_SA(sizeof(JwArgs_00030002V1) == 16 && JW_ALIGN(JwArgs_00030002V1) == 8, "args 00030002");
JW_SA(offsetof(JwArgs_00030002V1, bytes) == 0, "field 00030002.bytes");
JW_SA(sizeof(JwOut_00030002V1) == 48, "out 00030002");
/* std.net.SocketAddress.create */
typedef struct { JwSliceV1 ip; uint16_t port; uint32_t scope; } JwArgs_00030003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030003V1;
int32_t jw_net_socket_address_create_v1(void *context, JwArgs_00030003V1 *inout_args, JwOut_00030003V1 *out);
JW_SA(sizeof(JwArgs_00030003V1) == 24 && JW_ALIGN(JwArgs_00030003V1) == 8, "args 00030003");
JW_SA(offsetof(JwArgs_00030003V1, ip) == 0, "field 00030003.ip");
JW_SA(offsetof(JwArgs_00030003V1, port) == 16, "field 00030003.port");
JW_SA(offsetof(JwArgs_00030003V1, scope) == 20, "field 00030003.scope");
JW_SA(sizeof(JwOut_00030003V1) == 48, "out 00030003");
/* std.net.SocketAddress.ip */
typedef struct { JwSliceV1 self; } JwArgs_00030004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030004V1;
int32_t jw_net_socket_address_ip_v1(void *context, JwArgs_00030004V1 *inout_args, JwOut_00030004V1 *out);
JW_SA(sizeof(JwArgs_00030004V1) == 16 && JW_ALIGN(JwArgs_00030004V1) == 8, "args 00030004");
JW_SA(offsetof(JwArgs_00030004V1, self) == 0, "field 00030004.self");
JW_SA(sizeof(JwOut_00030004V1) == 48, "out 00030004");
/* std.net.SocketAddress.port */
typedef struct { JwSliceV1 self; } JwArgs_00030005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030005V1;
int32_t jw_net_socket_address_port_v1(void *context, JwArgs_00030005V1 *inout_args, JwOut_00030005V1 *out);
JW_SA(sizeof(JwArgs_00030005V1) == 16 && JW_ALIGN(JwArgs_00030005V1) == 8, "args 00030005");
JW_SA(offsetof(JwArgs_00030005V1, self) == 0, "field 00030005.self");
JW_SA(sizeof(JwOut_00030005V1) == 48, "out 00030005");
/* std.net.SocketAddress.scope */
typedef struct { JwSliceV1 self; } JwArgs_00030006V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030006V1;
int32_t jw_net_socket_address_scope_v1(void *context, JwArgs_00030006V1 *inout_args, JwOut_00030006V1 *out);
JW_SA(sizeof(JwArgs_00030006V1) == 16 && JW_ALIGN(JwArgs_00030006V1) == 8, "args 00030006");
JW_SA(offsetof(JwArgs_00030006V1, self) == 0, "field 00030006.self");
JW_SA(sizeof(JwOut_00030006V1) == 48, "out 00030006");
/* std.net.BindOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00030007V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030007V1;
int32_t jw_net_bind_options_defaults_v1(void *context, JwArgs_00030007V1 *inout_args, JwOut_00030007V1 *out);
JW_SA(sizeof(JwArgs_00030007V1) == 8 && JW_ALIGN(JwArgs_00030007V1) == 8, "args 00030007");
JW_SA(sizeof(JwOut_00030007V1) == 48, "out 00030007");
/* std.net.resolve */
typedef struct { JwSliceV1 host; uint16_t port; JwSliceV1 deadline; } JwArgs_00030008V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030008V1;
int32_t jw_net_resolve_v1(void *context, JwArgs_00030008V1 *inout_args, JwOut_00030008V1 *out);
JW_SA(sizeof(JwArgs_00030008V1) == 40 && JW_ALIGN(JwArgs_00030008V1) == 8, "args 00030008");
JW_SA(offsetof(JwArgs_00030008V1, host) == 0, "field 00030008.host");
JW_SA(offsetof(JwArgs_00030008V1, port) == 16, "field 00030008.port");
JW_SA(offsetof(JwArgs_00030008V1, deadline) == 24, "field 00030008.deadline");
JW_SA(sizeof(JwOut_00030008V1) == 48, "out 00030008");
/* std.net.connect */
typedef struct { JwSliceV1 target; JwSliceV1 deadline; } JwArgs_00030009V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00030009V1;
int32_t jw_net_connect_v1(void *context, JwArgs_00030009V1 *inout_args, JwOut_00030009V1 *out);
JW_SA(sizeof(JwArgs_00030009V1) == 32 && JW_ALIGN(JwArgs_00030009V1) == 8, "args 00030009");
JW_SA(offsetof(JwArgs_00030009V1, target) == 0, "field 00030009.target");
JW_SA(offsetof(JwArgs_00030009V1, deadline) == 16, "field 00030009.deadline");
JW_SA(sizeof(JwOut_00030009V1) == 72, "out 00030009");
/* std.net.bind */
typedef struct { JwSliceV1 address; JwSliceV1 options; } JwArgs_0003000aV1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_0003000aV1;
int32_t jw_net_bind_v1(void *context, JwArgs_0003000aV1 *inout_args, JwOut_0003000aV1 *out);
JW_SA(sizeof(JwArgs_0003000aV1) == 32 && JW_ALIGN(JwArgs_0003000aV1) == 8, "args 0003000a");
JW_SA(offsetof(JwArgs_0003000aV1, address) == 0, "field 0003000a.address");
JW_SA(offsetof(JwArgs_0003000aV1, options) == 16, "field 0003000a.options");
JW_SA(sizeof(JwOut_0003000aV1) == 72, "out 0003000a");
/* std.net.TcpListener.accept */
typedef struct { JwResTokenV1 self; JwSliceV1 deadline; } JwArgs_0003000bV1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_0003000bV1;
int32_t jw_net_tcp_listener_accept_v1(void *context, JwArgs_0003000bV1 *inout_args, JwOut_0003000bV1 *out);
JW_SA(sizeof(JwArgs_0003000bV1) == 40 && JW_ALIGN(JwArgs_0003000bV1) == 8, "args 0003000b");
JW_SA(offsetof(JwArgs_0003000bV1, self) == 0, "field 0003000b.self");
JW_SA(offsetof(JwArgs_0003000bV1, deadline) == 24, "field 0003000b.deadline");
JW_SA(sizeof(JwOut_0003000bV1) == 72, "out 0003000b");
/* std.net.TcpListener.address */
typedef struct { JwResTokenV1 self; } JwArgs_0003000cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0003000cV1;
int32_t jw_net_tcp_listener_address_v1(void *context, JwArgs_0003000cV1 *inout_args, JwOut_0003000cV1 *out);
JW_SA(sizeof(JwArgs_0003000cV1) == 24 && JW_ALIGN(JwArgs_0003000cV1) == 8, "args 0003000c");
JW_SA(offsetof(JwArgs_0003000cV1, self) == 0, "field 0003000c.self");
JW_SA(sizeof(JwOut_0003000cV1) == 48, "out 0003000c");
/* std.net.TcpListener.close */
typedef struct { JwResTokenV1 self; } JwArgs_0003000dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0003000dV1;
int32_t jw_net_tcp_listener_close_v1(void *context, JwArgs_0003000dV1 *inout_args, JwOut_0003000dV1 *out);
JW_SA(sizeof(JwArgs_0003000dV1) == 24 && JW_ALIGN(JwArgs_0003000dV1) == 8, "args 0003000d");
JW_SA(offsetof(JwArgs_0003000dV1, self) == 0, "field 0003000d.self");
JW_SA(sizeof(JwOut_0003000dV1) == 48, "out 0003000d");
/* std.net.TcpStream.read */
typedef struct { JwResTokenV1 self; uint64_t max_bytes; JwSliceV1 deadline; } JwArgs_0003000eV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0003000eV1;
int32_t jw_net_tcp_stream_read_v1(void *context, JwArgs_0003000eV1 *inout_args, JwOut_0003000eV1 *out);
JW_SA(sizeof(JwArgs_0003000eV1) == 48 && JW_ALIGN(JwArgs_0003000eV1) == 8, "args 0003000e");
JW_SA(offsetof(JwArgs_0003000eV1, self) == 0, "field 0003000e.self");
JW_SA(offsetof(JwArgs_0003000eV1, max_bytes) == 24, "field 0003000e.max_bytes");
JW_SA(offsetof(JwArgs_0003000eV1, deadline) == 32, "field 0003000e.deadline");
JW_SA(sizeof(JwOut_0003000eV1) == 48, "out 0003000e");
/* std.net.TcpStream.read_exact */
typedef struct { JwResTokenV1 self; uint64_t bytes; JwSliceV1 deadline; } JwArgs_0003000fV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0003000fV1;
int32_t jw_net_tcp_stream_read_exact_v1(void *context, JwArgs_0003000fV1 *inout_args, JwOut_0003000fV1 *out);
JW_SA(sizeof(JwArgs_0003000fV1) == 48 && JW_ALIGN(JwArgs_0003000fV1) == 8, "args 0003000f");
JW_SA(offsetof(JwArgs_0003000fV1, self) == 0, "field 0003000f.self");
JW_SA(offsetof(JwArgs_0003000fV1, bytes) == 24, "field 0003000f.bytes");
JW_SA(offsetof(JwArgs_0003000fV1, deadline) == 32, "field 0003000f.deadline");
JW_SA(sizeof(JwOut_0003000fV1) == 48, "out 0003000f");
/* std.net.TcpStream.write */
typedef struct { JwResTokenV1 self; JwSliceV1 data; JwSliceV1 deadline; } JwArgs_00030010V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030010V1;
int32_t jw_net_tcp_stream_write_v1(void *context, JwArgs_00030010V1 *inout_args, JwOut_00030010V1 *out);
JW_SA(sizeof(JwArgs_00030010V1) == 56 && JW_ALIGN(JwArgs_00030010V1) == 8, "args 00030010");
JW_SA(offsetof(JwArgs_00030010V1, self) == 0, "field 00030010.self");
JW_SA(offsetof(JwArgs_00030010V1, data) == 24, "field 00030010.data");
JW_SA(offsetof(JwArgs_00030010V1, deadline) == 40, "field 00030010.deadline");
JW_SA(sizeof(JwOut_00030010V1) == 48, "out 00030010");
/* std.net.TcpStream.write_all */
typedef struct { JwResTokenV1 self; JwSliceV1 data; JwSliceV1 deadline; } JwArgs_00030011V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030011V1;
int32_t jw_net_tcp_stream_write_all_v1(void *context, JwArgs_00030011V1 *inout_args, JwOut_00030011V1 *out);
JW_SA(sizeof(JwArgs_00030011V1) == 56 && JW_ALIGN(JwArgs_00030011V1) == 8, "args 00030011");
JW_SA(offsetof(JwArgs_00030011V1, self) == 0, "field 00030011.self");
JW_SA(offsetof(JwArgs_00030011V1, data) == 24, "field 00030011.data");
JW_SA(offsetof(JwArgs_00030011V1, deadline) == 40, "field 00030011.deadline");
JW_SA(sizeof(JwOut_00030011V1) == 48, "out 00030011");
/* std.net.TcpStream.local_address */
typedef struct { JwResTokenV1 self; } JwArgs_00030012V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030012V1;
int32_t jw_net_tcp_stream_local_address_v1(void *context, JwArgs_00030012V1 *inout_args, JwOut_00030012V1 *out);
JW_SA(sizeof(JwArgs_00030012V1) == 24 && JW_ALIGN(JwArgs_00030012V1) == 8, "args 00030012");
JW_SA(offsetof(JwArgs_00030012V1, self) == 0, "field 00030012.self");
JW_SA(sizeof(JwOut_00030012V1) == 48, "out 00030012");
/* std.net.TcpStream.peer_address */
typedef struct { JwResTokenV1 self; } JwArgs_00030013V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030013V1;
int32_t jw_net_tcp_stream_peer_address_v1(void *context, JwArgs_00030013V1 *inout_args, JwOut_00030013V1 *out);
JW_SA(sizeof(JwArgs_00030013V1) == 24 && JW_ALIGN(JwArgs_00030013V1) == 8, "args 00030013");
JW_SA(offsetof(JwArgs_00030013V1, self) == 0, "field 00030013.self");
JW_SA(sizeof(JwOut_00030013V1) == 48, "out 00030013");
/* std.net.TcpStream.shutdown_write */
typedef struct { JwResTokenV1 self; } JwArgs_00030014V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030014V1;
int32_t jw_net_tcp_stream_shutdown_write_v1(void *context, JwArgs_00030014V1 *inout_args, JwOut_00030014V1 *out);
JW_SA(sizeof(JwArgs_00030014V1) == 24 && JW_ALIGN(JwArgs_00030014V1) == 8, "args 00030014");
JW_SA(offsetof(JwArgs_00030014V1, self) == 0, "field 00030014.self");
JW_SA(sizeof(JwOut_00030014V1) == 48, "out 00030014");
/* std.net.TcpStream.close */
typedef struct { JwResTokenV1 self; } JwArgs_00030015V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00030015V1;
int32_t jw_net_tcp_stream_close_v1(void *context, JwArgs_00030015V1 *inout_args, JwOut_00030015V1 *out);
JW_SA(sizeof(JwArgs_00030015V1) == 24 && JW_ALIGN(JwArgs_00030015V1) == 8, "args 00030015");
JW_SA(offsetof(JwArgs_00030015V1, self) == 0, "field 00030015.self");
JW_SA(sizeof(JwOut_00030015V1) == 48, "out 00030015");
/* std.tls.Options.defaults */
typedef struct { uint64_t reserved; } JwArgs_00040001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00040001V1;
int32_t jw_tls_options_defaults_v1(void *context, JwArgs_00040001V1 *inout_args, JwOut_00040001V1 *out);
JW_SA(sizeof(JwArgs_00040001V1) == 8 && JW_ALIGN(JwArgs_00040001V1) == 8, "args 00040001");
JW_SA(sizeof(JwOut_00040001V1) == 48, "out 00040001");
/* std.tls.connect */
typedef struct { JwSliceV1 target; JwSliceV1 name; JwSliceV1 options; JwSliceV1 deadline; } JwArgs_00040002V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00040002V1;
int32_t jw_tls_connect_v1(void *context, JwArgs_00040002V1 *inout_args, JwOut_00040002V1 *out);
JW_SA(sizeof(JwArgs_00040002V1) == 64 && JW_ALIGN(JwArgs_00040002V1) == 8, "args 00040002");
JW_SA(offsetof(JwArgs_00040002V1, target) == 0, "field 00040002.target");
JW_SA(offsetof(JwArgs_00040002V1, name) == 16, "field 00040002.name");
JW_SA(offsetof(JwArgs_00040002V1, options) == 32, "field 00040002.options");
JW_SA(offsetof(JwArgs_00040002V1, deadline) == 48, "field 00040002.deadline");
JW_SA(sizeof(JwOut_00040002V1) == 72, "out 00040002");
/* std.tls.start_tls */
typedef struct { JwResTokenV1 stream; JwSliceV1 name; JwSliceV1 options; JwSliceV1 deadline; } JwArgs_00040003V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00040003V1;
int32_t jw_tls_start_tls_v1(void *context, JwArgs_00040003V1 *inout_args, JwOut_00040003V1 *out);
JW_SA(sizeof(JwArgs_00040003V1) == 72 && JW_ALIGN(JwArgs_00040003V1) == 8, "args 00040003");
JW_SA(offsetof(JwArgs_00040003V1, stream) == 0, "field 00040003.stream");
JW_SA(offsetof(JwArgs_00040003V1, name) == 24, "field 00040003.name");
JW_SA(offsetof(JwArgs_00040003V1, options) == 40, "field 00040003.options");
JW_SA(offsetof(JwArgs_00040003V1, deadline) == 56, "field 00040003.deadline");
JW_SA(sizeof(JwOut_00040003V1) == 72, "out 00040003");
/* std.tls.TlsStream.read */
typedef struct { JwResTokenV1 self; uint64_t max_bytes; JwSliceV1 deadline; } JwArgs_00040004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00040004V1;
int32_t jw_tls_tls_stream_read_v1(void *context, JwArgs_00040004V1 *inout_args, JwOut_00040004V1 *out);
JW_SA(sizeof(JwArgs_00040004V1) == 48 && JW_ALIGN(JwArgs_00040004V1) == 8, "args 00040004");
JW_SA(offsetof(JwArgs_00040004V1, self) == 0, "field 00040004.self");
JW_SA(offsetof(JwArgs_00040004V1, max_bytes) == 24, "field 00040004.max_bytes");
JW_SA(offsetof(JwArgs_00040004V1, deadline) == 32, "field 00040004.deadline");
JW_SA(sizeof(JwOut_00040004V1) == 48, "out 00040004");
/* std.tls.TlsStream.write */
typedef struct { JwResTokenV1 self; JwSliceV1 data; JwSliceV1 deadline; } JwArgs_00040005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00040005V1;
int32_t jw_tls_tls_stream_write_v1(void *context, JwArgs_00040005V1 *inout_args, JwOut_00040005V1 *out);
JW_SA(sizeof(JwArgs_00040005V1) == 56 && JW_ALIGN(JwArgs_00040005V1) == 8, "args 00040005");
JW_SA(offsetof(JwArgs_00040005V1, self) == 0, "field 00040005.self");
JW_SA(offsetof(JwArgs_00040005V1, data) == 24, "field 00040005.data");
JW_SA(offsetof(JwArgs_00040005V1, deadline) == 40, "field 00040005.deadline");
JW_SA(sizeof(JwOut_00040005V1) == 48, "out 00040005");
/* std.tls.TlsStream.write_all */
typedef struct { JwResTokenV1 self; JwSliceV1 data; JwSliceV1 deadline; } JwArgs_00040006V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00040006V1;
int32_t jw_tls_tls_stream_write_all_v1(void *context, JwArgs_00040006V1 *inout_args, JwOut_00040006V1 *out);
JW_SA(sizeof(JwArgs_00040006V1) == 56 && JW_ALIGN(JwArgs_00040006V1) == 8, "args 00040006");
JW_SA(offsetof(JwArgs_00040006V1, self) == 0, "field 00040006.self");
JW_SA(offsetof(JwArgs_00040006V1, data) == 24, "field 00040006.data");
JW_SA(offsetof(JwArgs_00040006V1, deadline) == 40, "field 00040006.deadline");
JW_SA(sizeof(JwOut_00040006V1) == 48, "out 00040006");
/* std.tls.TlsStream.close */
typedef struct { JwResTokenV1 self; JwSliceV1 deadline; } JwArgs_00040007V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00040007V1;
int32_t jw_tls_tls_stream_close_v1(void *context, JwArgs_00040007V1 *inout_args, JwOut_00040007V1 *out);
JW_SA(sizeof(JwArgs_00040007V1) == 40 && JW_ALIGN(JwArgs_00040007V1) == 8, "args 00040007");
JW_SA(offsetof(JwArgs_00040007V1, self) == 0, "field 00040007.self");
JW_SA(offsetof(JwArgs_00040007V1, deadline) == 24, "field 00040007.deadline");
JW_SA(sizeof(JwOut_00040007V1) == 48, "out 00040007");
/* postgres.connect */
typedef struct { JwSliceV1 options; } JwArgs_00050001V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00050001V1;
int32_t jw_postgres_connect_v1(void *context, JwArgs_00050001V1 *inout_args, JwOut_00050001V1 *out);
JW_SA(sizeof(JwArgs_00050001V1) == 16 && JW_ALIGN(JwArgs_00050001V1) == 8, "args 00050001");
JW_SA(offsetof(JwArgs_00050001V1, options) == 0, "field 00050001.options");
JW_SA(sizeof(JwOut_00050001V1) == 72, "out 00050001");
/* postgres.QueryOptions.defaults */
typedef struct { JwSliceV1 deadline; } JwArgs_00050002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00050002V1;
int32_t jw_postgres_query_options_defaults_v1(void *context, JwArgs_00050002V1 *inout_args, JwOut_00050002V1 *out);
JW_SA(sizeof(JwArgs_00050002V1) == 16 && JW_ALIGN(JwArgs_00050002V1) == 8, "args 00050002");
JW_SA(offsetof(JwArgs_00050002V1, deadline) == 0, "field 00050002.deadline");
JW_SA(sizeof(JwOut_00050002V1) == 48, "out 00050002");
/* postgres.TxOptions.defaults */
typedef struct { JwSliceV1 deadline; } JwArgs_00050003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00050003V1;
int32_t jw_postgres_tx_options_defaults_v1(void *context, JwArgs_00050003V1 *inout_args, JwOut_00050003V1 *out);
JW_SA(sizeof(JwArgs_00050003V1) == 16 && JW_ALIGN(JwArgs_00050003V1) == 8, "args 00050003");
JW_SA(offsetof(JwArgs_00050003V1, deadline) == 0, "field 00050003.deadline");
JW_SA(sizeof(JwOut_00050003V1) == 48, "out 00050003");
/* postgres.Connection.run */
typedef struct { JwResTokenV1 self; JwSliceV1 sql; JwSliceV1 params; JwSliceV1 options; } JwArgs_00050004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00050004V1;
int32_t jw_postgres_connection_run_v1(void *context, JwArgs_00050004V1 *inout_args, JwOut_00050004V1 *out);
JW_SA(sizeof(JwArgs_00050004V1) == 72 && JW_ALIGN(JwArgs_00050004V1) == 8, "args 00050004");
JW_SA(offsetof(JwArgs_00050004V1, self) == 0, "field 00050004.self");
JW_SA(offsetof(JwArgs_00050004V1, sql) == 24, "field 00050004.sql");
JW_SA(offsetof(JwArgs_00050004V1, params) == 40, "field 00050004.params");
JW_SA(offsetof(JwArgs_00050004V1, options) == 56, "field 00050004.options");
JW_SA(sizeof(JwOut_00050004V1) == 48, "out 00050004");
/* postgres.Transaction.run */
typedef struct { JwResTokenV1 self; JwSliceV1 sql; JwSliceV1 params; JwSliceV1 options; } JwArgs_00050005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00050005V1;
int32_t jw_postgres_transaction_run_v1(void *context, JwArgs_00050005V1 *inout_args, JwOut_00050005V1 *out);
JW_SA(sizeof(JwArgs_00050005V1) == 72 && JW_ALIGN(JwArgs_00050005V1) == 8, "args 00050005");
JW_SA(offsetof(JwArgs_00050005V1, self) == 0, "field 00050005.self");
JW_SA(offsetof(JwArgs_00050005V1, sql) == 24, "field 00050005.sql");
JW_SA(offsetof(JwArgs_00050005V1, params) == 40, "field 00050005.params");
JW_SA(offsetof(JwArgs_00050005V1, options) == 56, "field 00050005.options");
JW_SA(sizeof(JwOut_00050005V1) == 48, "out 00050005");
/* postgres.Connection.begin */
typedef struct { JwResTokenV1 self; JwSliceV1 options; } JwArgs_00050006V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00050006V1;
int32_t jw_postgres_connection_begin_v1(void *context, JwArgs_00050006V1 *inout_args, JwOut_00050006V1 *out);
JW_SA(sizeof(JwArgs_00050006V1) == 40 && JW_ALIGN(JwArgs_00050006V1) == 8, "args 00050006");
JW_SA(offsetof(JwArgs_00050006V1, self) == 0, "field 00050006.self");
JW_SA(offsetof(JwArgs_00050006V1, options) == 24, "field 00050006.options");
JW_SA(sizeof(JwOut_00050006V1) == 72, "out 00050006");
/* postgres.Connection.close */
typedef struct { JwResTokenV1 self; } JwArgs_00050007V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00050007V1;
int32_t jw_postgres_connection_close_v1(void *context, JwArgs_00050007V1 *inout_args, JwOut_00050007V1 *out);
JW_SA(sizeof(JwArgs_00050007V1) == 24 && JW_ALIGN(JwArgs_00050007V1) == 8, "args 00050007");
JW_SA(offsetof(JwArgs_00050007V1, self) == 0, "field 00050007.self");
JW_SA(sizeof(JwOut_00050007V1) == 48, "out 00050007");
/* postgres.Transaction.commit */
typedef struct { JwResTokenV1 self; JwSliceV1 deadline; } JwArgs_00050008V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00050008V1;
int32_t jw_postgres_transaction_commit_v1(void *context, JwArgs_00050008V1 *inout_args, JwOut_00050008V1 *out);
JW_SA(sizeof(JwArgs_00050008V1) == 40 && JW_ALIGN(JwArgs_00050008V1) == 8, "args 00050008");
JW_SA(offsetof(JwArgs_00050008V1, self) == 0, "field 00050008.self");
JW_SA(offsetof(JwArgs_00050008V1, deadline) == 24, "field 00050008.deadline");
JW_SA(sizeof(JwOut_00050008V1) == 72, "out 00050008");
/* postgres.Transaction.rollback */
typedef struct { JwResTokenV1 self; JwSliceV1 deadline; } JwArgs_00050009V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00050009V1;
int32_t jw_postgres_transaction_rollback_v1(void *context, JwArgs_00050009V1 *inout_args, JwOut_00050009V1 *out);
JW_SA(sizeof(JwArgs_00050009V1) == 40 && JW_ALIGN(JwArgs_00050009V1) == 8, "args 00050009");
JW_SA(offsetof(JwArgs_00050009V1, self) == 0, "field 00050009.self");
JW_SA(offsetof(JwArgs_00050009V1, deadline) == 24, "field 00050009.deadline");
JW_SA(sizeof(JwOut_00050009V1) == 72, "out 00050009");
/* postgres.QueryResult.column_index */
typedef struct { JwSliceV1 self; JwSliceV1 name; } JwArgs_0005000aV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0005000aV1;
int32_t jw_postgres_query_result_column_index_v1(void *context, JwArgs_0005000aV1 *inout_args, JwOut_0005000aV1 *out);
JW_SA(sizeof(JwArgs_0005000aV1) == 32 && JW_ALIGN(JwArgs_0005000aV1) == 8, "args 0005000a");
JW_SA(offsetof(JwArgs_0005000aV1, self) == 0, "field 0005000a.self");
JW_SA(offsetof(JwArgs_0005000aV1, name) == 16, "field 0005000a.name");
JW_SA(sizeof(JwOut_0005000aV1) == 48, "out 0005000a");
/* postgres.Row.cell */
typedef struct { JwSliceV1 self; uint32_t index; } JwArgs_0005000bV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0005000bV1;
int32_t jw_postgres_row_cell_v1(void *context, JwArgs_0005000bV1 *inout_args, JwOut_0005000bV1 *out);
JW_SA(sizeof(JwArgs_0005000bV1) == 24 && JW_ALIGN(JwArgs_0005000bV1) == 8, "args 0005000b");
JW_SA(offsetof(JwArgs_0005000bV1, self) == 0, "field 0005000b.self");
JW_SA(offsetof(JwArgs_0005000bV1, index) == 16, "field 0005000b.index");
JW_SA(sizeof(JwOut_0005000bV1) == 48, "out 0005000b");
/* postgres.Cell.as_bool */
typedef struct { JwSliceV1 self; } JwArgs_0005000cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0005000cV1;
int32_t jw_postgres_cell_as_bool_v1(void *context, JwArgs_0005000cV1 *inout_args, JwOut_0005000cV1 *out);
JW_SA(sizeof(JwArgs_0005000cV1) == 16 && JW_ALIGN(JwArgs_0005000cV1) == 8, "args 0005000c");
JW_SA(offsetof(JwArgs_0005000cV1, self) == 0, "field 0005000c.self");
JW_SA(sizeof(JwOut_0005000cV1) == 48, "out 0005000c");
/* postgres.Cell.as_i64 */
typedef struct { JwSliceV1 self; } JwArgs_0005000dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0005000dV1;
int32_t jw_postgres_cell_as_i64_v1(void *context, JwArgs_0005000dV1 *inout_args, JwOut_0005000dV1 *out);
JW_SA(sizeof(JwArgs_0005000dV1) == 16 && JW_ALIGN(JwArgs_0005000dV1) == 8, "args 0005000d");
JW_SA(offsetof(JwArgs_0005000dV1, self) == 0, "field 0005000d.self");
JW_SA(sizeof(JwOut_0005000dV1) == 48, "out 0005000d");
/* postgres.Cell.as_f64 */
typedef struct { JwSliceV1 self; } JwArgs_0005000eV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0005000eV1;
int32_t jw_postgres_cell_as_f64_v1(void *context, JwArgs_0005000eV1 *inout_args, JwOut_0005000eV1 *out);
JW_SA(sizeof(JwArgs_0005000eV1) == 16 && JW_ALIGN(JwArgs_0005000eV1) == 8, "args 0005000e");
JW_SA(offsetof(JwArgs_0005000eV1, self) == 0, "field 0005000e.self");
JW_SA(sizeof(JwOut_0005000eV1) == 48, "out 0005000e");
/* postgres.Cell.as_text */
typedef struct { JwSliceV1 self; } JwArgs_0005000fV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0005000fV1;
int32_t jw_postgres_cell_as_text_v1(void *context, JwArgs_0005000fV1 *inout_args, JwOut_0005000fV1 *out);
JW_SA(sizeof(JwArgs_0005000fV1) == 16 && JW_ALIGN(JwArgs_0005000fV1) == 8, "args 0005000f");
JW_SA(offsetof(JwArgs_0005000fV1, self) == 0, "field 0005000f.self");
JW_SA(sizeof(JwOut_0005000fV1) == 48, "out 0005000f");
/* postgres.Cell.as_bytes */
typedef struct { JwSliceV1 self; } JwArgs_00050010V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00050010V1;
int32_t jw_postgres_cell_as_bytes_v1(void *context, JwArgs_00050010V1 *inout_args, JwOut_00050010V1 *out);
JW_SA(sizeof(JwArgs_00050010V1) == 16 && JW_ALIGN(JwArgs_00050010V1) == 8, "args 00050010");
JW_SA(offsetof(JwArgs_00050010V1, self) == 0, "field 00050010.self");
JW_SA(sizeof(JwOut_00050010V1) == 48, "out 00050010");
/* window.Options.defaults */
typedef struct { uint64_t reserved; } JwArgs_00060001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060001V1;
int32_t jw_window_options_defaults_v1(void *context, JwArgs_00060001V1 *inout_args, JwOut_00060001V1 *out);
JW_SA(sizeof(JwArgs_00060001V1) == 8 && JW_ALIGN(JwArgs_00060001V1) == 8, "args 00060001");
JW_SA(sizeof(JwOut_00060001V1) == 48, "out 00060001");
/* window.create_window */
typedef struct { JwSliceV1 options; } JwArgs_00060002V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00060002V1;
int32_t jw_window_create_window_v1(void *context, JwArgs_00060002V1 *inout_args, JwOut_00060002V1 *out);
JW_SA(sizeof(JwArgs_00060002V1) == 16 && JW_ALIGN(JwArgs_00060002V1) == 8, "args 00060002");
JW_SA(offsetof(JwArgs_00060002V1, options) == 0, "field 00060002.options");
JW_SA(sizeof(JwOut_00060002V1) == 72, "out 00060002");
/* window.Window.poll_events */
typedef struct { JwResTokenV1 self; } JwArgs_00060003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060003V1;
int32_t jw_window_window_poll_events_v1(void *context, JwArgs_00060003V1 *inout_args, JwOut_00060003V1 *out);
JW_SA(sizeof(JwArgs_00060003V1) == 24 && JW_ALIGN(JwArgs_00060003V1) == 8, "args 00060003");
JW_SA(offsetof(JwArgs_00060003V1, self) == 0, "field 00060003.self");
JW_SA(sizeof(JwOut_00060003V1) == 48, "out 00060003");
/* window.Window.wait_events */
typedef struct { JwResTokenV1 self; JwSliceV1 deadline; } JwArgs_00060004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060004V1;
int32_t jw_window_window_wait_events_v1(void *context, JwArgs_00060004V1 *inout_args, JwOut_00060004V1 *out);
JW_SA(sizeof(JwArgs_00060004V1) == 40 && JW_ALIGN(JwArgs_00060004V1) == 8, "args 00060004");
JW_SA(offsetof(JwArgs_00060004V1, self) == 0, "field 00060004.self");
JW_SA(offsetof(JwArgs_00060004V1, deadline) == 24, "field 00060004.deadline");
JW_SA(sizeof(JwOut_00060004V1) == 48, "out 00060004");
/* window.Window.info */
typedef struct { JwResTokenV1 self; } JwArgs_00060005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060005V1;
int32_t jw_window_window_info_v1(void *context, JwArgs_00060005V1 *inout_args, JwOut_00060005V1 *out);
JW_SA(sizeof(JwArgs_00060005V1) == 24 && JW_ALIGN(JwArgs_00060005V1) == 8, "args 00060005");
JW_SA(offsetof(JwArgs_00060005V1, self) == 0, "field 00060005.self");
JW_SA(sizeof(JwOut_00060005V1) == 48, "out 00060005");
/* window.Window.request_redraw */
typedef struct { JwResTokenV1 self; } JwArgs_00060006V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060006V1;
int32_t jw_window_window_request_redraw_v1(void *context, JwArgs_00060006V1 *inout_args, JwOut_00060006V1 *out);
JW_SA(sizeof(JwArgs_00060006V1) == 24 && JW_ALIGN(JwArgs_00060006V1) == 8, "args 00060006");
JW_SA(offsetof(JwArgs_00060006V1, self) == 0, "field 00060006.self");
JW_SA(sizeof(JwOut_00060006V1) == 48, "out 00060006");
/* window.Window.close */
typedef struct { JwResTokenV1 self; } JwArgs_00060007V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060007V1;
int32_t jw_window_window_close_v1(void *context, JwArgs_00060007V1 *inout_args, JwOut_00060007V1 *out);
JW_SA(sizeof(JwArgs_00060007V1) == 24 && JW_ALIGN(JwArgs_00060007V1) == 8, "args 00060007");
JW_SA(offsetof(JwArgs_00060007V1, self) == 0, "field 00060007.self");
JW_SA(sizeof(JwOut_00060007V1) == 48, "out 00060007");
/* window.EventBatch.events */
typedef struct { JwSliceV1 self; } JwArgs_00060008V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060008V1;
int32_t jw_window_event_batch_events_v1(void *context, JwArgs_00060008V1 *inout_args, JwOut_00060008V1 *out);
JW_SA(sizeof(JwArgs_00060008V1) == 16 && JW_ALIGN(JwArgs_00060008V1) == 8, "args 00060008");
JW_SA(offsetof(JwArgs_00060008V1, self) == 0, "field 00060008.self");
JW_SA(sizeof(JwOut_00060008V1) == 48, "out 00060008");
/* window.GameInputBatch.events */
typedef struct { JwSliceV1 self; } JwArgs_00060009V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00060009V1;
int32_t jw_window_game_input_batch_events_v1(void *context, JwArgs_00060009V1 *inout_args, JwOut_00060009V1 *out);
JW_SA(sizeof(JwArgs_00060009V1) == 16 && JW_ALIGN(JwArgs_00060009V1) == 8, "args 00060009");
JW_SA(offsetof(JwArgs_00060009V1, self) == 0, "field 00060009.self");
JW_SA(sizeof(JwOut_00060009V1) == 48, "out 00060009");
/* draw2d.RendererOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00070001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070001V1;
int32_t jw_draw2d_renderer_options_defaults_v1(void *context, JwArgs_00070001V1 *inout_args, JwOut_00070001V1 *out);
JW_SA(sizeof(JwArgs_00070001V1) == 8 && JW_ALIGN(JwArgs_00070001V1) == 8, "args 00070001");
JW_SA(sizeof(JwOut_00070001V1) == 48, "out 00070001");
/* draw2d.TextureOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00070002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070002V1;
int32_t jw_draw2d_texture_options_defaults_v1(void *context, JwArgs_00070002V1 *inout_args, JwOut_00070002V1 *out);
JW_SA(sizeof(JwArgs_00070002V1) == 8 && JW_ALIGN(JwArgs_00070002V1) == 8, "args 00070002");
JW_SA(sizeof(JwOut_00070002V1) == 48, "out 00070002");
/* draw2d.TextStyle.defaults */
typedef struct { uint64_t reserved; } JwArgs_00070003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070003V1;
int32_t jw_draw2d_text_style_defaults_v1(void *context, JwArgs_00070003V1 *inout_args, JwOut_00070003V1 *out);
JW_SA(sizeof(JwArgs_00070003V1) == 8 && JW_ALIGN(JwArgs_00070003V1) == 8, "args 00070003");
JW_SA(sizeof(JwOut_00070003V1) == 48, "out 00070003");
/* draw2d.PresentOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00070004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070004V1;
int32_t jw_draw2d_present_options_defaults_v1(void *context, JwArgs_00070004V1 *inout_args, JwOut_00070004V1 *out);
JW_SA(sizeof(JwArgs_00070004V1) == 8 && JW_ALIGN(JwArgs_00070004V1) == 8, "args 00070004");
JW_SA(sizeof(JwOut_00070004V1) == 48, "out 00070004");
/* draw2d.create_renderer */
typedef struct { JwResTokenV1 window; JwSliceV1 options; } JwArgs_00070005V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00070005V1;
int32_t jw_draw2d_create_renderer_v1(void *context, JwArgs_00070005V1 *inout_args, JwOut_00070005V1 *out);
JW_SA(sizeof(JwArgs_00070005V1) == 40 && JW_ALIGN(JwArgs_00070005V1) == 8, "args 00070005");
JW_SA(offsetof(JwArgs_00070005V1, window) == 0, "field 00070005.window");
JW_SA(offsetof(JwArgs_00070005V1, options) == 24, "field 00070005.options");
JW_SA(sizeof(JwOut_00070005V1) == 72, "out 00070005");
/* draw2d.create_text_layout */
typedef struct { JwSliceV1 text; JwSliceV1 options; } JwArgs_00070006V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00070006V1;
int32_t jw_draw2d_create_text_layout_v1(void *context, JwArgs_00070006V1 *inout_args, JwOut_00070006V1 *out);
JW_SA(sizeof(JwArgs_00070006V1) == 32 && JW_ALIGN(JwArgs_00070006V1) == 8, "args 00070006");
JW_SA(offsetof(JwArgs_00070006V1, text) == 0, "field 00070006.text");
JW_SA(offsetof(JwArgs_00070006V1, options) == 16, "field 00070006.options");
JW_SA(sizeof(JwOut_00070006V1) == 72, "out 00070006");
/* draw2d.Texture.from_rgba8 */
typedef struct { JwResTokenV1 renderer; JwSliceV1 size; JwSliceV1 pixels; JwSliceV1 options; } JwArgs_00070007V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00070007V1;
int32_t jw_draw2d_texture_from_rgba8_v1(void *context, JwArgs_00070007V1 *inout_args, JwOut_00070007V1 *out);
JW_SA(sizeof(JwArgs_00070007V1) == 72 && JW_ALIGN(JwArgs_00070007V1) == 8, "args 00070007");
JW_SA(offsetof(JwArgs_00070007V1, renderer) == 0, "field 00070007.renderer");
JW_SA(offsetof(JwArgs_00070007V1, size) == 24, "field 00070007.size");
JW_SA(offsetof(JwArgs_00070007V1, pixels) == 40, "field 00070007.pixels");
JW_SA(offsetof(JwArgs_00070007V1, options) == 56, "field 00070007.options");
JW_SA(sizeof(JwOut_00070007V1) == 72, "out 00070007");
/* draw2d.Renderer.info */
typedef struct { JwResTokenV1 self; } JwArgs_00070008V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070008V1;
int32_t jw_draw2d_renderer_info_v1(void *context, JwArgs_00070008V1 *inout_args, JwOut_00070008V1 *out);
JW_SA(sizeof(JwArgs_00070008V1) == 24 && JW_ALIGN(JwArgs_00070008V1) == 8, "args 00070008");
JW_SA(offsetof(JwArgs_00070008V1, self) == 0, "field 00070008.self");
JW_SA(sizeof(JwOut_00070008V1) == 48, "out 00070008");
/* draw2d.Renderer.begin_frame */
typedef struct { JwResTokenV1 self; JwSliceV1 clear; } JwArgs_00070009V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00070009V1;
int32_t jw_draw2d_renderer_begin_frame_v1(void *context, JwArgs_00070009V1 *inout_args, JwOut_00070009V1 *out);
JW_SA(sizeof(JwArgs_00070009V1) == 40 && JW_ALIGN(JwArgs_00070009V1) == 8, "args 00070009");
JW_SA(offsetof(JwArgs_00070009V1, self) == 0, "field 00070009.self");
JW_SA(offsetof(JwArgs_00070009V1, clear) == 24, "field 00070009.clear");
JW_SA(sizeof(JwOut_00070009V1) == 72, "out 00070009");
/* draw2d.Renderer.close */
typedef struct { JwResTokenV1 self; } JwArgs_0007000aV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0007000aV1;
int32_t jw_draw2d_renderer_close_v1(void *context, JwArgs_0007000aV1 *inout_args, JwOut_0007000aV1 *out);
JW_SA(sizeof(JwArgs_0007000aV1) == 24 && JW_ALIGN(JwArgs_0007000aV1) == 8, "args 0007000a");
JW_SA(offsetof(JwArgs_0007000aV1, self) == 0, "field 0007000a.self");
JW_SA(sizeof(JwOut_0007000aV1) == 48, "out 0007000a");
/* draw2d.Texture.close */
typedef struct { JwResTokenV1 self; } JwArgs_0007000bV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0007000bV1;
int32_t jw_draw2d_texture_close_v1(void *context, JwArgs_0007000bV1 *inout_args, JwOut_0007000bV1 *out);
JW_SA(sizeof(JwArgs_0007000bV1) == 24 && JW_ALIGN(JwArgs_0007000bV1) == 8, "args 0007000b");
JW_SA(offsetof(JwArgs_0007000bV1, self) == 0, "field 0007000b.self");
JW_SA(sizeof(JwOut_0007000bV1) == 48, "out 0007000b");
/* draw2d.TextLayout.close */
typedef struct { JwResTokenV1 self; } JwArgs_0007000cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0007000cV1;
int32_t jw_draw2d_text_layout_close_v1(void *context, JwArgs_0007000cV1 *inout_args, JwOut_0007000cV1 *out);
JW_SA(sizeof(JwArgs_0007000cV1) == 24 && JW_ALIGN(JwArgs_0007000cV1) == 8, "args 0007000c");
JW_SA(offsetof(JwArgs_0007000cV1, self) == 0, "field 0007000c.self");
JW_SA(sizeof(JwOut_0007000cV1) == 48, "out 0007000c");
/* draw2d.TextLayout.info */
typedef struct { JwResTokenV1 self; } JwArgs_0007000dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0007000dV1;
int32_t jw_draw2d_text_layout_info_v1(void *context, JwArgs_0007000dV1 *inout_args, JwOut_0007000dV1 *out);
JW_SA(sizeof(JwArgs_0007000dV1) == 24 && JW_ALIGN(JwArgs_0007000dV1) == 8, "args 0007000d");
JW_SA(offsetof(JwArgs_0007000dV1, self) == 0, "field 0007000d.self");
JW_SA(sizeof(JwOut_0007000dV1) == 48, "out 0007000d");
/* draw2d.Frame.fill_rect */
typedef struct { JwResTokenV1 self; JwSliceV1 rect; JwSliceV1 color; } JwArgs_0007000eV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0007000eV1;
int32_t jw_draw2d_frame_fill_rect_v1(void *context, JwArgs_0007000eV1 *inout_args, JwOut_0007000eV1 *out);
JW_SA(sizeof(JwArgs_0007000eV1) == 56 && JW_ALIGN(JwArgs_0007000eV1) == 8, "args 0007000e");
JW_SA(offsetof(JwArgs_0007000eV1, self) == 0, "field 0007000e.self");
JW_SA(offsetof(JwArgs_0007000eV1, rect) == 24, "field 0007000e.rect");
JW_SA(offsetof(JwArgs_0007000eV1, color) == 40, "field 0007000e.color");
JW_SA(sizeof(JwOut_0007000eV1) == 48, "out 0007000e");
/* draw2d.Frame.stroke_rect */
typedef struct { JwResTokenV1 self; JwSliceV1 rect; JwSliceV1 color; double width; } JwArgs_0007000fV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0007000fV1;
int32_t jw_draw2d_frame_stroke_rect_v1(void *context, JwArgs_0007000fV1 *inout_args, JwOut_0007000fV1 *out);
JW_SA(sizeof(JwArgs_0007000fV1) == 64 && JW_ALIGN(JwArgs_0007000fV1) == 8, "args 0007000f");
JW_SA(offsetof(JwArgs_0007000fV1, self) == 0, "field 0007000f.self");
JW_SA(offsetof(JwArgs_0007000fV1, rect) == 24, "field 0007000f.rect");
JW_SA(offsetof(JwArgs_0007000fV1, color) == 40, "field 0007000f.color");
JW_SA(offsetof(JwArgs_0007000fV1, width) == 56, "field 0007000f.width");
JW_SA(sizeof(JwOut_0007000fV1) == 48, "out 0007000f");
/* draw2d.Frame.triangle */
typedef struct { JwResTokenV1 self; JwSliceV1 a; JwSliceV1 b; JwSliceV1 c; JwSliceV1 color; } JwArgs_00070010V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070010V1;
int32_t jw_draw2d_frame_triangle_v1(void *context, JwArgs_00070010V1 *inout_args, JwOut_00070010V1 *out);
JW_SA(sizeof(JwArgs_00070010V1) == 88 && JW_ALIGN(JwArgs_00070010V1) == 8, "args 00070010");
JW_SA(offsetof(JwArgs_00070010V1, self) == 0, "field 00070010.self");
JW_SA(offsetof(JwArgs_00070010V1, a) == 24, "field 00070010.a");
JW_SA(offsetof(JwArgs_00070010V1, b) == 40, "field 00070010.b");
JW_SA(offsetof(JwArgs_00070010V1, c) == 56, "field 00070010.c");
JW_SA(offsetof(JwArgs_00070010V1, color) == 72, "field 00070010.color");
JW_SA(sizeof(JwOut_00070010V1) == 48, "out 00070010");
/* draw2d.Frame.image */
typedef struct { JwResTokenV1 self; JwResTokenV1 texture; JwSliceV1 source; JwSliceV1 target; JwSliceV1 tint; } JwArgs_00070011V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070011V1;
int32_t jw_draw2d_frame_image_v1(void *context, JwArgs_00070011V1 *inout_args, JwOut_00070011V1 *out);
JW_SA(sizeof(JwArgs_00070011V1) == 96 && JW_ALIGN(JwArgs_00070011V1) == 8, "args 00070011");
JW_SA(offsetof(JwArgs_00070011V1, self) == 0, "field 00070011.self");
JW_SA(offsetof(JwArgs_00070011V1, texture) == 24, "field 00070011.texture");
JW_SA(offsetof(JwArgs_00070011V1, source) == 48, "field 00070011.source");
JW_SA(offsetof(JwArgs_00070011V1, target) == 64, "field 00070011.target");
JW_SA(offsetof(JwArgs_00070011V1, tint) == 80, "field 00070011.tint");
JW_SA(sizeof(JwOut_00070011V1) == 48, "out 00070011");
/* draw2d.Frame.text */
typedef struct { JwResTokenV1 self; JwResTokenV1 layout; JwSliceV1 origin; JwSliceV1 color; } JwArgs_00070012V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070012V1;
int32_t jw_draw2d_frame_text_v1(void *context, JwArgs_00070012V1 *inout_args, JwOut_00070012V1 *out);
JW_SA(sizeof(JwArgs_00070012V1) == 80 && JW_ALIGN(JwArgs_00070012V1) == 8, "args 00070012");
JW_SA(offsetof(JwArgs_00070012V1, self) == 0, "field 00070012.self");
JW_SA(offsetof(JwArgs_00070012V1, layout) == 24, "field 00070012.layout");
JW_SA(offsetof(JwArgs_00070012V1, origin) == 48, "field 00070012.origin");
JW_SA(offsetof(JwArgs_00070012V1, color) == 64, "field 00070012.color");
JW_SA(sizeof(JwOut_00070012V1) == 48, "out 00070012");
/* draw2d.Frame.push_clip */
typedef struct { JwResTokenV1 self; JwSliceV1 rect; } JwArgs_00070013V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070013V1;
int32_t jw_draw2d_frame_push_clip_v1(void *context, JwArgs_00070013V1 *inout_args, JwOut_00070013V1 *out);
JW_SA(sizeof(JwArgs_00070013V1) == 40 && JW_ALIGN(JwArgs_00070013V1) == 8, "args 00070013");
JW_SA(offsetof(JwArgs_00070013V1, self) == 0, "field 00070013.self");
JW_SA(offsetof(JwArgs_00070013V1, rect) == 24, "field 00070013.rect");
JW_SA(sizeof(JwOut_00070013V1) == 48, "out 00070013");
/* draw2d.Frame.pop_clip */
typedef struct { JwResTokenV1 self; } JwArgs_00070014V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070014V1;
int32_t jw_draw2d_frame_pop_clip_v1(void *context, JwArgs_00070014V1 *inout_args, JwOut_00070014V1 *out);
JW_SA(sizeof(JwArgs_00070014V1) == 24 && JW_ALIGN(JwArgs_00070014V1) == 8, "args 00070014");
JW_SA(offsetof(JwArgs_00070014V1, self) == 0, "field 00070014.self");
JW_SA(sizeof(JwOut_00070014V1) == 48, "out 00070014");
/* draw2d.Frame.push_transform */
typedef struct { JwResTokenV1 self; JwSliceV1 transform; } JwArgs_00070015V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070015V1;
int32_t jw_draw2d_frame_push_transform_v1(void *context, JwArgs_00070015V1 *inout_args, JwOut_00070015V1 *out);
JW_SA(sizeof(JwArgs_00070015V1) == 40 && JW_ALIGN(JwArgs_00070015V1) == 8, "args 00070015");
JW_SA(offsetof(JwArgs_00070015V1, self) == 0, "field 00070015.self");
JW_SA(offsetof(JwArgs_00070015V1, transform) == 24, "field 00070015.transform");
JW_SA(sizeof(JwOut_00070015V1) == 48, "out 00070015");
/* draw2d.Frame.pop_transform */
typedef struct { JwResTokenV1 self; } JwArgs_00070016V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070016V1;
int32_t jw_draw2d_frame_pop_transform_v1(void *context, JwArgs_00070016V1 *inout_args, JwOut_00070016V1 *out);
JW_SA(sizeof(JwArgs_00070016V1) == 24 && JW_ALIGN(JwArgs_00070016V1) == 8, "args 00070016");
JW_SA(offsetof(JwArgs_00070016V1, self) == 0, "field 00070016.self");
JW_SA(sizeof(JwOut_00070016V1) == 48, "out 00070016");
/* draw2d.Frame.finish */
typedef struct { JwResTokenV1 self; JwSliceV1 options; } JwArgs_00070017V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070017V1;
int32_t jw_draw2d_frame_finish_v1(void *context, JwArgs_00070017V1 *inout_args, JwOut_00070017V1 *out);
JW_SA(sizeof(JwArgs_00070017V1) == 40 && JW_ALIGN(JwArgs_00070017V1) == 8, "args 00070017");
JW_SA(offsetof(JwArgs_00070017V1, self) == 0, "field 00070017.self");
JW_SA(offsetof(JwArgs_00070017V1, options) == 24, "field 00070017.options");
JW_SA(sizeof(JwOut_00070017V1) == 48, "out 00070017");
/* draw2d.Frame.abort */
typedef struct { JwResTokenV1 self; } JwArgs_00070018V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00070018V1;
int32_t jw_draw2d_frame_abort_v1(void *context, JwArgs_00070018V1 *inout_args, JwOut_00070018V1 *out);
JW_SA(sizeof(JwArgs_00070018V1) == 24 && JW_ALIGN(JwArgs_00070018V1) == 8, "args 00070018");
JW_SA(offsetof(JwArgs_00070018V1, self) == 0, "field 00070018.self");
JW_SA(sizeof(JwOut_00070018V1) == 48, "out 00070018");
/* image.Limits.defaults */
typedef struct { uint64_t reserved; } JwArgs_00080001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00080001V1;
int32_t jw_image_limits_defaults_v1(void *context, JwArgs_00080001V1 *inout_args, JwOut_00080001V1 *out);
JW_SA(sizeof(JwArgs_00080001V1) == 8 && JW_ALIGN(JwArgs_00080001V1) == 8, "args 00080001");
JW_SA(sizeof(JwOut_00080001V1) == 48, "out 00080001");
/* image.decode */
typedef struct { JwSliceV1 encoded; JwSliceV1 limits; } JwArgs_00080002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00080002V1;
int32_t jw_image_decode_v1(void *context, JwArgs_00080002V1 *inout_args, JwOut_00080002V1 *out);
JW_SA(sizeof(JwArgs_00080002V1) == 32 && JW_ALIGN(JwArgs_00080002V1) == 8, "args 00080002");
JW_SA(offsetof(JwArgs_00080002V1, encoded) == 0, "field 00080002.encoded");
JW_SA(offsetof(JwArgs_00080002V1, limits) == 16, "field 00080002.limits");
JW_SA(sizeof(JwOut_00080002V1) == 48, "out 00080002");
/* image.load */
typedef struct { JwSliceV1 path; JwSliceV1 limits; } JwArgs_00080003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00080003V1;
int32_t jw_image_load_v1(void *context, JwArgs_00080003V1 *inout_args, JwOut_00080003V1 *out);
JW_SA(sizeof(JwArgs_00080003V1) == 32 && JW_ALIGN(JwArgs_00080003V1) == 8, "args 00080003");
JW_SA(offsetof(JwArgs_00080003V1, path) == 0, "field 00080003.path");
JW_SA(offsetof(JwArgs_00080003V1, limits) == 16, "field 00080003.limits");
JW_SA(sizeof(JwOut_00080003V1) == 48, "out 00080003");
/* gui.Insets.defaults */
typedef struct { uint64_t reserved; } JwArgs_00090001V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090001V1;
int32_t jw_gui_insets_defaults_v1(void *context, JwArgs_00090001V1 *inout_args, JwOut_00090001V1 *out);
JW_SA(sizeof(JwArgs_00090001V1) == 8 && JW_ALIGN(JwArgs_00090001V1) == 8, "args 00090001");
JW_SA(sizeof(JwOut_00090001V1) == 48, "out 00090001");
/* gui.Layout.defaults */
typedef struct { uint64_t reserved; } JwArgs_00090002V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090002V1;
int32_t jw_gui_layout_defaults_v1(void *context, JwArgs_00090002V1 *inout_args, JwOut_00090002V1 *out);
JW_SA(sizeof(JwArgs_00090002V1) == 8 && JW_ALIGN(JwArgs_00090002V1) == 8, "args 00090002");
JW_SA(sizeof(JwOut_00090002V1) == 48, "out 00090002");
/* gui.Semantics.defaults */
typedef struct { uint64_t reserved; } JwArgs_00090003V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090003V1;
int32_t jw_gui_semantics_defaults_v1(void *context, JwArgs_00090003V1 *inout_args, JwOut_00090003V1 *out);
JW_SA(sizeof(JwArgs_00090003V1) == 8 && JW_ALIGN(JwArgs_00090003V1) == 8, "args 00090003");
JW_SA(sizeof(JwOut_00090003V1) == 48, "out 00090003");
/* gui.TextInputOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00090004V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090004V1;
int32_t jw_gui_text_input_options_defaults_v1(void *context, JwArgs_00090004V1 *inout_args, JwOut_00090004V1 *out);
JW_SA(sizeof(JwArgs_00090004V1) == 8 && JW_ALIGN(JwArgs_00090004V1) == 8, "args 00090004");
JW_SA(sizeof(JwOut_00090004V1) == 48, "out 00090004");
/* gui.GuiLimits.defaults */
typedef struct { uint64_t reserved; } JwArgs_00090005V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090005V1;
int32_t jw_gui_gui_limits_defaults_v1(void *context, JwArgs_00090005V1 *inout_args, JwOut_00090005V1 *out);
JW_SA(sizeof(JwArgs_00090005V1) == 8 && JW_ALIGN(JwArgs_00090005V1) == 8, "args 00090005");
JW_SA(sizeof(JwOut_00090005V1) == 48, "out 00090005");
/* gui.GuiOptions.defaults */
typedef struct { uint64_t reserved; } JwArgs_00090006V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090006V1;
int32_t jw_gui_gui_options_defaults_v1(void *context, JwArgs_00090006V1 *inout_args, JwOut_00090006V1 *out);
JW_SA(sizeof(JwArgs_00090006V1) == 8 && JW_ALIGN(JwArgs_00090006V1) == 8, "args 00090006");
JW_SA(sizeof(JwOut_00090006V1) == 48, "out 00090006");
/* gui.create_gui */
typedef struct { JwResTokenV1 window; JwSliceV1 options; } JwArgs_00090007V1;
typedef struct { JwOutcomeV1 result; JwResTokenV1 resource_0; } JwOut_00090007V1;
int32_t jw_gui_create_gui_v1(void *context, JwArgs_00090007V1 *inout_args, JwOut_00090007V1 *out);
JW_SA(sizeof(JwArgs_00090007V1) == 40 && JW_ALIGN(JwArgs_00090007V1) == 8, "args 00090007");
JW_SA(offsetof(JwArgs_00090007V1, window) == 0, "field 00090007.window");
JW_SA(offsetof(JwArgs_00090007V1, options) == 24, "field 00090007.options");
JW_SA(sizeof(JwOut_00090007V1) == 72, "out 00090007");
/* gui.GuiContext.root */
typedef struct { JwResTokenV1 self; } JwArgs_00090008V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090008V1;
int32_t jw_gui_gui_context_root_v1(void *context, JwArgs_00090008V1 *inout_args, JwOut_00090008V1 *out);
JW_SA(sizeof(JwArgs_00090008V1) == 24 && JW_ALIGN(JwArgs_00090008V1) == 8, "args 00090008");
JW_SA(offsetof(JwArgs_00090008V1, self) == 0, "field 00090008.self");
JW_SA(sizeof(JwOut_00090008V1) == 48, "out 00090008");
/* gui.GuiContext.create */
typedef struct { JwResTokenV1 self; JwSliceV1 parent; JwSliceV1 spec; JwSliceV1 layout; JwSliceV1 semantics; } JwArgs_00090009V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090009V1;
int32_t jw_gui_gui_context_create_v1(void *context, JwArgs_00090009V1 *inout_args, JwOut_00090009V1 *out);
JW_SA(sizeof(JwArgs_00090009V1) == 88 && JW_ALIGN(JwArgs_00090009V1) == 8, "args 00090009");
JW_SA(offsetof(JwArgs_00090009V1, self) == 0, "field 00090009.self");
JW_SA(offsetof(JwArgs_00090009V1, parent) == 24, "field 00090009.parent");
JW_SA(offsetof(JwArgs_00090009V1, spec) == 40, "field 00090009.spec");
JW_SA(offsetof(JwArgs_00090009V1, layout) == 56, "field 00090009.layout");
JW_SA(offsetof(JwArgs_00090009V1, semantics) == 72, "field 00090009.semantics");
JW_SA(sizeof(JwOut_00090009V1) == 48, "out 00090009");
/* gui.GuiContext.reparent */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 parent; uint32_t index; } JwArgs_0009000aV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009000aV1;
int32_t jw_gui_gui_context_reparent_v1(void *context, JwArgs_0009000aV1 *inout_args, JwOut_0009000aV1 *out);
JW_SA(sizeof(JwArgs_0009000aV1) == 64 && JW_ALIGN(JwArgs_0009000aV1) == 8, "args 0009000a");
JW_SA(offsetof(JwArgs_0009000aV1, self) == 0, "field 0009000a.self");
JW_SA(offsetof(JwArgs_0009000aV1, node) == 24, "field 0009000a.node");
JW_SA(offsetof(JwArgs_0009000aV1, parent) == 40, "field 0009000a.parent");
JW_SA(offsetof(JwArgs_0009000aV1, index) == 56, "field 0009000a.index");
JW_SA(sizeof(JwOut_0009000aV1) == 48, "out 0009000a");
/* gui.GuiContext.remove */
typedef struct { JwResTokenV1 self; JwSliceV1 node; } JwArgs_0009000bV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009000bV1;
int32_t jw_gui_gui_context_remove_v1(void *context, JwArgs_0009000bV1 *inout_args, JwOut_0009000bV1 *out);
JW_SA(sizeof(JwArgs_0009000bV1) == 40 && JW_ALIGN(JwArgs_0009000bV1) == 8, "args 0009000b");
JW_SA(offsetof(JwArgs_0009000bV1, self) == 0, "field 0009000b.self");
JW_SA(offsetof(JwArgs_0009000bV1, node) == 24, "field 0009000b.node");
JW_SA(sizeof(JwOut_0009000bV1) == 48, "out 0009000b");
/* gui.GuiContext.set_layout */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_0009000cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009000cV1;
int32_t jw_gui_gui_context_set_layout_v1(void *context, JwArgs_0009000cV1 *inout_args, JwOut_0009000cV1 *out);
JW_SA(sizeof(JwArgs_0009000cV1) == 56 && JW_ALIGN(JwArgs_0009000cV1) == 8, "args 0009000c");
JW_SA(offsetof(JwArgs_0009000cV1, self) == 0, "field 0009000c.self");
JW_SA(offsetof(JwArgs_0009000cV1, node) == 24, "field 0009000c.node");
JW_SA(offsetof(JwArgs_0009000cV1, value) == 40, "field 0009000c.value");
JW_SA(sizeof(JwOut_0009000cV1) == 48, "out 0009000c");
/* gui.GuiContext.set_style */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_0009000dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009000dV1;
int32_t jw_gui_gui_context_set_style_v1(void *context, JwArgs_0009000dV1 *inout_args, JwOut_0009000dV1 *out);
JW_SA(sizeof(JwArgs_0009000dV1) == 56 && JW_ALIGN(JwArgs_0009000dV1) == 8, "args 0009000d");
JW_SA(offsetof(JwArgs_0009000dV1, self) == 0, "field 0009000d.self");
JW_SA(offsetof(JwArgs_0009000dV1, node) == 24, "field 0009000d.node");
JW_SA(offsetof(JwArgs_0009000dV1, value) == 40, "field 0009000d.value");
JW_SA(sizeof(JwOut_0009000dV1) == 48, "out 0009000d");
/* gui.GuiContext.set_text_style */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_0009000eV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009000eV1;
int32_t jw_gui_gui_context_set_text_style_v1(void *context, JwArgs_0009000eV1 *inout_args, JwOut_0009000eV1 *out);
JW_SA(sizeof(JwArgs_0009000eV1) == 56 && JW_ALIGN(JwArgs_0009000eV1) == 8, "args 0009000e");
JW_SA(offsetof(JwArgs_0009000eV1, self) == 0, "field 0009000e.self");
JW_SA(offsetof(JwArgs_0009000eV1, node) == 24, "field 0009000e.node");
JW_SA(offsetof(JwArgs_0009000eV1, value) == 40, "field 0009000e.value");
JW_SA(sizeof(JwOut_0009000eV1) == 48, "out 0009000e");
/* gui.GuiContext.set_input_options */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_0009000fV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009000fV1;
int32_t jw_gui_gui_context_set_input_options_v1(void *context, JwArgs_0009000fV1 *inout_args, JwOut_0009000fV1 *out);
JW_SA(sizeof(JwArgs_0009000fV1) == 56 && JW_ALIGN(JwArgs_0009000fV1) == 8, "args 0009000f");
JW_SA(offsetof(JwArgs_0009000fV1, self) == 0, "field 0009000f.self");
JW_SA(offsetof(JwArgs_0009000fV1, node) == 24, "field 0009000f.node");
JW_SA(offsetof(JwArgs_0009000fV1, value) == 40, "field 0009000f.value");
JW_SA(sizeof(JwOut_0009000fV1) == 48, "out 0009000f");
/* gui.GuiContext.set_semantics */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_00090010V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090010V1;
int32_t jw_gui_gui_context_set_semantics_v1(void *context, JwArgs_00090010V1 *inout_args, JwOut_00090010V1 *out);
JW_SA(sizeof(JwArgs_00090010V1) == 56 && JW_ALIGN(JwArgs_00090010V1) == 8, "args 00090010");
JW_SA(offsetof(JwArgs_00090010V1, self) == 0, "field 00090010.self");
JW_SA(offsetof(JwArgs_00090010V1, node) == 24, "field 00090010.node");
JW_SA(offsetof(JwArgs_00090010V1, value) == 40, "field 00090010.value");
JW_SA(sizeof(JwOut_00090010V1) == 48, "out 00090010");
/* gui.GuiContext.set_visibility */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_00090011V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090011V1;
int32_t jw_gui_gui_context_set_visibility_v1(void *context, JwArgs_00090011V1 *inout_args, JwOut_00090011V1 *out);
JW_SA(sizeof(JwArgs_00090011V1) == 56 && JW_ALIGN(JwArgs_00090011V1) == 8, "args 00090011");
JW_SA(offsetof(JwArgs_00090011V1, self) == 0, "field 00090011.self");
JW_SA(offsetof(JwArgs_00090011V1, node) == 24, "field 00090011.node");
JW_SA(offsetof(JwArgs_00090011V1, value) == 40, "field 00090011.value");
JW_SA(sizeof(JwOut_00090011V1) == 48, "out 00090011");
/* gui.GuiContext.set_selection */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 value; } JwArgs_00090012V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090012V1;
int32_t jw_gui_gui_context_set_selection_v1(void *context, JwArgs_00090012V1 *inout_args, JwOut_00090012V1 *out);
JW_SA(sizeof(JwArgs_00090012V1) == 56 && JW_ALIGN(JwArgs_00090012V1) == 8, "args 00090012");
JW_SA(offsetof(JwArgs_00090012V1, self) == 0, "field 00090012.self");
JW_SA(offsetof(JwArgs_00090012V1, node) == 24, "field 00090012.node");
JW_SA(offsetof(JwArgs_00090012V1, value) == 40, "field 00090012.value");
JW_SA(sizeof(JwOut_00090012V1) == 48, "out 00090012");
/* gui.GuiContext.set_gap */
typedef struct { JwResTokenV1 self; JwSliceV1 node; double value; } JwArgs_00090013V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090013V1;
int32_t jw_gui_gui_context_set_gap_v1(void *context, JwArgs_00090013V1 *inout_args, JwOut_00090013V1 *out);
JW_SA(sizeof(JwArgs_00090013V1) == 48 && JW_ALIGN(JwArgs_00090013V1) == 8, "args 00090013");
JW_SA(offsetof(JwArgs_00090013V1, self) == 0, "field 00090013.self");
JW_SA(offsetof(JwArgs_00090013V1, node) == 24, "field 00090013.node");
JW_SA(offsetof(JwArgs_00090013V1, value) == 40, "field 00090013.value");
JW_SA(sizeof(JwOut_00090013V1) == 48, "out 00090013");
/* gui.GuiContext.set_block_background */
typedef struct { JwResTokenV1 self; JwSliceV1 node; uint32_t value; } JwArgs_00090014V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090014V1;
int32_t jw_gui_gui_context_set_block_background_v1(void *context, JwArgs_00090014V1 *inout_args, JwOut_00090014V1 *out);
JW_SA(sizeof(JwArgs_00090014V1) == 48 && JW_ALIGN(JwArgs_00090014V1) == 8, "args 00090014");
JW_SA(offsetof(JwArgs_00090014V1, self) == 0, "field 00090014.self");
JW_SA(offsetof(JwArgs_00090014V1, node) == 24, "field 00090014.node");
JW_SA(offsetof(JwArgs_00090014V1, value) == 40, "field 00090014.value");
JW_SA(sizeof(JwOut_00090014V1) == 48, "out 00090014");
/* gui.GuiContext.set_enabled */
typedef struct { JwResTokenV1 self; JwSliceV1 node; uint32_t value; } JwArgs_00090015V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090015V1;
int32_t jw_gui_gui_context_set_enabled_v1(void *context, JwArgs_00090015V1 *inout_args, JwOut_00090015V1 *out);
JW_SA(sizeof(JwArgs_00090015V1) == 48 && JW_ALIGN(JwArgs_00090015V1) == 8, "args 00090015");
JW_SA(offsetof(JwArgs_00090015V1, self) == 0, "field 00090015.self");
JW_SA(offsetof(JwArgs_00090015V1, node) == 24, "field 00090015.node");
JW_SA(offsetof(JwArgs_00090015V1, value) == 40, "field 00090015.value");
JW_SA(sizeof(JwOut_00090015V1) == 48, "out 00090015");
/* gui.GuiContext.set_text */
typedef struct { JwResTokenV1 self; JwSliceV1 node; JwSliceV1 text; } JwArgs_00090016V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090016V1;
int32_t jw_gui_gui_context_set_text_v1(void *context, JwArgs_00090016V1 *inout_args, JwOut_00090016V1 *out);
JW_SA(sizeof(JwArgs_00090016V1) == 56 && JW_ALIGN(JwArgs_00090016V1) == 8, "args 00090016");
JW_SA(offsetof(JwArgs_00090016V1, self) == 0, "field 00090016.self");
JW_SA(offsetof(JwArgs_00090016V1, node) == 24, "field 00090016.node");
JW_SA(offsetof(JwArgs_00090016V1, text) == 40, "field 00090016.text");
JW_SA(sizeof(JwOut_00090016V1) == 48, "out 00090016");
/* gui.GuiContext.text */
typedef struct { JwResTokenV1 self; JwSliceV1 node; } JwArgs_00090017V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090017V1;
int32_t jw_gui_gui_context_text_v1(void *context, JwArgs_00090017V1 *inout_args, JwOut_00090017V1 *out);
JW_SA(sizeof(JwArgs_00090017V1) == 40 && JW_ALIGN(JwArgs_00090017V1) == 8, "args 00090017");
JW_SA(offsetof(JwArgs_00090017V1, self) == 0, "field 00090017.self");
JW_SA(offsetof(JwArgs_00090017V1, node) == 24, "field 00090017.node");
JW_SA(sizeof(JwOut_00090017V1) == 48, "out 00090017");
/* gui.GuiContext.selection */
typedef struct { JwResTokenV1 self; JwSliceV1 node; } JwArgs_00090018V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090018V1;
int32_t jw_gui_gui_context_selection_v1(void *context, JwArgs_00090018V1 *inout_args, JwOut_00090018V1 *out);
JW_SA(sizeof(JwArgs_00090018V1) == 40 && JW_ALIGN(JwArgs_00090018V1) == 8, "args 00090018");
JW_SA(offsetof(JwArgs_00090018V1, self) == 0, "field 00090018.self");
JW_SA(offsetof(JwArgs_00090018V1, node) == 24, "field 00090018.node");
JW_SA(sizeof(JwOut_00090018V1) == 48, "out 00090018");
/* gui.GuiContext.composition */
typedef struct { JwResTokenV1 self; JwSliceV1 node; } JwArgs_00090019V1;
typedef struct { JwOutcomeV1 result;  } JwOut_00090019V1;
int32_t jw_gui_gui_context_composition_v1(void *context, JwArgs_00090019V1 *inout_args, JwOut_00090019V1 *out);
JW_SA(sizeof(JwArgs_00090019V1) == 40 && JW_ALIGN(JwArgs_00090019V1) == 8, "args 00090019");
JW_SA(offsetof(JwArgs_00090019V1, self) == 0, "field 00090019.self");
JW_SA(offsetof(JwArgs_00090019V1, node) == 24, "field 00090019.node");
JW_SA(sizeof(JwOut_00090019V1) == 48, "out 00090019");
/* gui.GuiContext.node_info */
typedef struct { JwResTokenV1 self; JwSliceV1 node; } JwArgs_0009001aV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009001aV1;
int32_t jw_gui_gui_context_node_info_v1(void *context, JwArgs_0009001aV1 *inout_args, JwOut_0009001aV1 *out);
JW_SA(sizeof(JwArgs_0009001aV1) == 40 && JW_ALIGN(JwArgs_0009001aV1) == 8, "args 0009001a");
JW_SA(offsetof(JwArgs_0009001aV1, self) == 0, "field 0009001a.self");
JW_SA(offsetof(JwArgs_0009001aV1, node) == 24, "field 0009001a.node");
JW_SA(sizeof(JwOut_0009001aV1) == 48, "out 0009001a");
/* gui.GuiContext.request_focus */
typedef struct { JwResTokenV1 self; JwSliceV1 node; } JwArgs_0009001bV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009001bV1;
int32_t jw_gui_gui_context_request_focus_v1(void *context, JwArgs_0009001bV1 *inout_args, JwOut_0009001bV1 *out);
JW_SA(sizeof(JwArgs_0009001bV1) == 40 && JW_ALIGN(JwArgs_0009001bV1) == 8, "args 0009001b");
JW_SA(offsetof(JwArgs_0009001bV1, self) == 0, "field 0009001b.self");
JW_SA(offsetof(JwArgs_0009001bV1, node) == 24, "field 0009001b.node");
JW_SA(sizeof(JwOut_0009001bV1) == 48, "out 0009001b");
/* gui.GuiContext.layout */
typedef struct { JwResTokenV1 self; } JwArgs_0009001cV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009001cV1;
int32_t jw_gui_gui_context_layout_v1(void *context, JwArgs_0009001cV1 *inout_args, JwOut_0009001cV1 *out);
JW_SA(sizeof(JwArgs_0009001cV1) == 24 && JW_ALIGN(JwArgs_0009001cV1) == 8, "args 0009001c");
JW_SA(offsetof(JwArgs_0009001cV1, self) == 0, "field 0009001c.self");
JW_SA(sizeof(JwOut_0009001cV1) == 48, "out 0009001c");
/* gui.GuiContext.update */
typedef struct { JwResTokenV1 self; JwSliceV1 batch; JwSliceV1 now; } JwArgs_0009001dV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009001dV1;
int32_t jw_gui_gui_context_update_v1(void *context, JwArgs_0009001dV1 *inout_args, JwOut_0009001dV1 *out);
JW_SA(sizeof(JwArgs_0009001dV1) == 56 && JW_ALIGN(JwArgs_0009001dV1) == 8, "args 0009001d");
JW_SA(offsetof(JwArgs_0009001dV1, self) == 0, "field 0009001d.self");
JW_SA(offsetof(JwArgs_0009001dV1, batch) == 24, "field 0009001d.batch");
JW_SA(offsetof(JwArgs_0009001dV1, now) == 40, "field 0009001d.now");
JW_SA(sizeof(JwOut_0009001dV1) == 48, "out 0009001d");
/* gui.GuiContext.draw */
typedef struct { JwResTokenV1 self; JwResTokenV1 frame; } JwArgs_0009001eV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009001eV1;
int32_t jw_gui_gui_context_draw_v1(void *context, JwArgs_0009001eV1 *inout_args, JwOut_0009001eV1 *out);
JW_SA(sizeof(JwArgs_0009001eV1) == 48 && JW_ALIGN(JwArgs_0009001eV1) == 8, "args 0009001e");
JW_SA(offsetof(JwArgs_0009001eV1, self) == 0, "field 0009001e.self");
JW_SA(offsetof(JwArgs_0009001eV1, frame) == 24, "field 0009001e.frame");
JW_SA(sizeof(JwOut_0009001eV1) == 48, "out 0009001e");
/* gui.GuiContext.close */
typedef struct { JwResTokenV1 self; } JwArgs_0009001fV1;
typedef struct { JwOutcomeV1 result;  } JwOut_0009001fV1;
int32_t jw_gui_gui_context_close_v1(void *context, JwArgs_0009001fV1 *inout_args, JwOut_0009001fV1 *out);
JW_SA(sizeof(JwArgs_0009001fV1) == 24 && JW_ALIGN(JwArgs_0009001fV1) == 8, "args 0009001f");
JW_SA(offsetof(JwArgs_0009001fV1, self) == 0, "field 0009001f.self");
JW_SA(sizeof(JwOut_0009001fV1) == 48, "out 0009001f");
#ifdef __cplusplus
}
#endif
#undef JW_SA
#undef JW_ALIGN
#endif
