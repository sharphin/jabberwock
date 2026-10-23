// ward の実行時ライブラリ。`ward build` が LLVM の書き出し（src/ir/llvm.rs）と一緒に clang でコンパイルする。
//
// - own の先は「区画」。区画の頭（16 バイト）に世代と「最後に解放された場所」を置く。解放しても OS には返さず、
//   同じ大きさの区画の置き場に戻すだけ（世代は進めたまま残るので、古い ref は必ず見つかる）。
// - ref は (指す先, 区画, 借りたときの世代, 借りた場所)。区画の世代と比べて確かめる（比べるのは書き出したコード）。
// - own の「読むだけの写し」（for の要素・クロージャが取り込んだ値）は、下の 1 ビットを立てたポインタで、
//   先は「写しの札」（区画, 世代, 借りた場所）。先をたどるときに世代を確かめる。
// - 文字列は書き換えない。数を数えて共有する（rc が負なら消えない定数）。
// - 実行時エラーは、出力をすべて書き出してから標準エラーに出し、終了コード 1 で終わる。

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// 位置（ファイル・行・列・長さ）と、その位置の説明。解放の場所なら chain は「連鎖して解放された」の位置
typedef struct WSite {
    const char *file;
    int32_t line, col, len, fileno;
    const char *text;
    const struct WSite *chain;
} WSite;

typedef struct WStr {
    int64_t rc;
    int64_t len;
    char data[];
} WStr;

typedef struct WList {
    char *data;
    int64_t len, cap;
} WList;

typedef struct WSb {
    char *p;
    int64_t len, cap;
} WSb;

// ---------------------------------------------------------------- 出力

static char *out_buf;
static size_t out_len, out_cap;

void ward_flush(void) {
    if (out_len) fwrite(out_buf, 1, out_len, stdout);
    out_len = 0;
    fflush(stdout);
}

static void out_write(const char *p, size_t n) {
    if (out_len + n > out_cap) {
        size_t cap = out_cap ? out_cap * 2 : 1 << 16;
        while (cap < out_len + n) cap *= 2;
        out_buf = realloc(out_buf, cap);
        out_cap = cap;
    }
    memcpy(out_buf + out_len, p, n);
    out_len += n;
    if (out_len >= 1 << 16) ward_flush();
}

// print の 1 つ分（first でなければ前に空白）
void ward_print_part(WStr *s, int32_t first) {
    if (!first) out_write(" ", 1);
    out_write(s->data, (size_t)s->len);
}

void ward_print_end(void) { out_write("\n", 1); }

// ---------------------------------------------------------------- エラー

static int is_cont_b(unsigned char c) { return (c & 0xC0) == 0x80; }

// 書き出したコードが置く: ファイルの名前と中身（エラーでソースの行を出すため。`--release` では空）
typedef struct WSrc {
    const char *name;
    const char *text;
} WSrc;
extern const WSrc ward_files[];
extern const int32_t ward_nfiles;

// エラーの位置を集めてから、`Diagnostic::render_in` と同じ形で出す
#define MAXL 8
static struct {
    const WSite *s;
    char text[600];
    int primary;
} labels[MAXL];
static int nlabels;
static char err_title[1024];

static void begin_error(const char *title) {
    ward_flush();
    snprintf(err_title, sizeof err_title, "%s", title);
    nlabels = 0;
}

static void put_site(const WSite *s, const char *label) {
    if (nlabels == MAXL) return;
    labels[nlabels].s = s;
    snprintf(labels[nlabels].text, sizeof labels[nlabels].text, "%s", label ? label : "");
    labels[nlabels].primary = 0;
    nlabels++;
}

// ソースの ln 行目（1 始まり）。Rust の `lines()` と同じ区切り
static const char *line_of(const char *src, int ln, int64_t *len) {
    const char *p = src;
    for (int i = 1; i < ln && *p; i++) {
        const char *nl = strchr(p, '\n');
        p = nl ? nl + 1 : p + strlen(p);
    }
    const char *e = strchr(p, '\n');
    int64_t n = e ? (int64_t)(e - p) : (int64_t)strlen(p);
    if (n > 0 && p[n - 1] == '\r') n--;
    *len = n;
    return p;
}

static void spaces(int n) {
    for (int i = 0; i < n; i++) fputc(' ', stderr);
}

static void arrow(int width, const WSite *s) {
    spaces(width);
    fprintf(stderr, "--> %s:%d:%d\n", s->file, s->line, s->col);
    spaces(width);
    fprintf(stderr, " |\n");
}

static int label_after(const WSite *a, const WSite *b, int main_file) {
    int ka = a->fileno != main_file, kb = b->fileno != main_file;
    if (ka != kb) return ka > kb;
    if (a->fileno != b->fileno) return a->fileno > b->fileno;
    if (a->line != b->line) return a->line > b->line;
    return a->col > b->col;
}

static _Noreturn void finish_error(const WSite *at, const char *label, const char *note) {
    put_site(at, label);
    labels[nlabels - 1].primary = 1;
    fprintf(stderr, "error: %s\n", err_title);
    if (ward_nfiles == 0) {
        // --release: 使った位置だけ
        fprintf(stderr, " --> %s:%d:%d\n", at->file, at->line, at->col);
        fprintf(stderr, "  = %s\n", label ? label : "");
        exit(1);
    }
    int order[MAXL], main_file = at->fileno;
    for (int i = 0; i < nlabels; i++) order[i] = i;
    for (int i = 1; i < nlabels; i++)
        for (int j = i; j > 0 && label_after(labels[order[j - 1]].s, labels[order[j]].s, main_file); j--) {
            int t = order[j];
            order[j] = order[j - 1];
            order[j - 1] = t;
        }
    int maxline = 1;
    for (int i = 0; i < nlabels; i++)
        if (labels[i].s->line > maxline) maxline = labels[i].s->line;
    char tmp[16];
    int width = snprintf(tmp, sizeof tmp, "%d", maxline);
    arrow(width, at);
    int last = 0, cur = main_file;
    for (int k = 0; k < nlabels; k++) {
        const WSite *s = labels[order[k]].s;
        if (s->fileno != cur) {
            // 別のファイルの位置
            cur = s->fileno;
            last = 0;
            arrow(width, s);
        }
        const char *src = cur < ward_nfiles ? ward_files[cur].text : ward_files[0].text;
        int64_t n;
        const char *line = line_of(src, s->line, &n);
        if (s->line != last) {
            if (last != 0 && s->line > last + 1) {
                spaces(width);
                fprintf(stderr, " ...\n");
            }
            fprintf(stderr, "%*d | ", width, s->line);
            for (int64_t i = 0; i < n; i++) fputc(line[i] == '\t' ? ' ' : line[i], stderr);
            fputc('\n', stderr);
            last = s->line;
        }
        // 下線: 列は文字単位。ASCII でない文字は 2 桁
        int lead = 0, mark = 0, col = 1, upto = s->col + (s->len > 0 ? s->len : 1);
        for (int64_t i = 0; i < n; col++) {
            int w = (unsigned char)line[i] < 0x80 ? 1 : 2;
            if (col < s->col) lead += w;
            else if (col < upto) mark += w;
            i++;
            while (i < n && is_cont_b((unsigned char)line[i])) i++;
        }
        if (mark < 1) mark = 1;
        spaces(width);
        fprintf(stderr, " | ");
        spaces(lead);
        for (int m = 0; m < mark; m++) fputc(labels[order[k]].primary ? '^' : '-', stderr);
        fprintf(stderr, " %s\n", labels[order[k]].text);
    }
    if (note) {
        spaces(width);
        fprintf(stderr, " |\n");
        spaces(width);
        fprintf(stderr, " = note: %s\n", note);
    }
    exit(1);
}

_Noreturn void ward_fail(const char *title, const WSite *at, const char *label, const char *note) {
    begin_error(title);
    finish_error(at, label, note);
}

// IR では来ないはずの場所（書き出しの誤り）
_Noreturn void ward_bug(const char *what) {
    ward_flush();
    fprintf(stderr, "内部エラー: %s\n", what);
    exit(2);
}

static int same_site(const WSite *a, const WSite *b) {
    return a->line == b->line && a->col == b->col && a->len == b->len && strcmp(a->file, b->file) == 0;
}

_Noreturn void ward_fail_moved(const char *name, const WSite *moved, const WSite *use) {
    char title[512], label[512];
    snprintf(title, sizeof title, "ムーブ済みの値 `%s` を使いました (use after move)", name);
    begin_error(title);
    if (same_site(moved, use)) {
        snprintf(label, sizeof label, "ここで `%s` の所有権を移すが、前の周回ですでに移動済み", name);
        finish_error(use, label, "ループの中で own を渡すと、2周目には手元に残っていません");
    }
    snprintf(label, sizeof label, "ここで `%s` の所有権が移動した", name);
    put_site(moved, label);
    snprintf(label, sizeof label, "ここで `%s` を使ったが、もう所有していない", name);
    finish_error(use, label, "移した後も使いたいなら `ref` で借りてください（例: `f(ref a)`）");
}

typedef struct WHdr {
    uint64_t gen;
    const WSite *freed;
} WHdr;

#define HDR(p) ((WHdr *)((char *)(p) - sizeof(WHdr)))

// 使い回した区画の、前の解放の位置（エラーで「ここで解放された」を出すため。古いものから消える）
#define NREUSED 4096
static struct {
    void *block;
    uint64_t gen;
    const WSite *freed;
} reused[NREUSED];
static unsigned reused_n;

// 世代 gen の ref が無効になった解放（世代 gen+1 になったとき）の位置
static const WSite *freed_at(void *block, uint64_t gen) {
    if (HDR(block)->gen == gen + 1) return HDR(block)->freed;
    for (unsigned k = 0; k < NREUSED; k++) {
        unsigned i = (reused_n - 1 - k) % NREUSED;
        if (reused[i].block == block && reused[i].gen == gen + 1) return reused[i].freed;
    }
    return NULL;
}

_Noreturn void ward_fail_dangling(const WSite *use, const WSite *borrowed, void *block, uint64_t gen) {
    char label[512];
    begin_error("無効になった参照を使いました (dangling reference)");
    snprintf(label, sizeof label, "ここで `%s` を借りた", borrowed->text);
    put_site(borrowed, label);
    const WSite *f = freed_at(block, gen);
    if (f) {
        snprintf(label, sizeof label, "ここで解放された: %s", f->text);
        put_site(f, label);
    }
    finish_error(use, "ここで使ったが、借りた先はもう存在しない", "参照は、借りた元の所有者より長くは生きられません");
}

static void fmt_int(char *buf, size_t n, int64_t v, int32_t is_unsigned) {
    if (is_unsigned) snprintf(buf, n, "%llu", (unsigned long long)v);
    else snprintf(buf, n, "%lld", (long long)v);
}

_Noreturn void ward_fail_bounds(int64_t i, int32_t is_unsigned, int64_t len, const WSite *at) {
    char title[256], label[256], num[32];
    fmt_int(num, sizeof num, i, is_unsigned);
    snprintf(title, sizeof title, "添字 %s は配列の範囲の外です", num);
    snprintf(label, sizeof label, "長さは %lld なので、添字は 0〜%lld です", (long long)len, (long long)len - 1);
    begin_error(title);
    finish_error(at, label, NULL);
}

_Noreturn void ward_fail_depth(const WSite *at) {
    begin_error("関数呼び出しが深すぎます");
    finish_error(at, "呼び出しの深さが 5000 を超えました", NULL);
}

// 辞書にキーがない（kind: 0 整数・1 文字列・2 真偽値）
_Noreturn void ward_fail_nokey(int32_t kind, int64_t key, int32_t is_unsigned, const WSite *at) {
    char title[1024], shown[900];
    if (kind == 1) {
        WStr *s = (WStr *)(intptr_t)key;
        snprintf(shown, sizeof shown, "\"%.*s\"", (int)(s->len > 800 ? 800 : s->len), s->data);
    } else if (kind == 2) {
        snprintf(shown, sizeof shown, "%s", key ? "true" : "false");
    } else {
        fmt_int(shown, sizeof shown, key, is_unsigned);
    }
    snprintf(title, sizeof title, "辞書にキー %s がありません", shown);
    begin_error(title);
    finish_error(at, "このキーの値は入っていない", "ないかもしれないなら `m.get(k)`（Option）か `m.contains(k)` を使ってください");
}

_Noreturn void ward_fail_insert(int64_t i, int64_t len, const WSite *at) {
    char label[256];
    snprintf(label, sizeof label, "位置は %lld ですが、長さは %lld です", (long long)i, (long long)len);
    begin_error("insert の位置が範囲の外です");
    finish_error(at, label, "入れられる位置は 0〜長さ です");
}

// ---------------------------------------------------------------- 区画

int64_t ward_live;
int64_t ward_depth;

#define NCLASS 256
static void *free_small[NCLASS + 1];
typedef struct Big {
    struct Big *next;
    size_t size;
    void *payload;
} Big;
static Big *free_big;

static size_t round16(int64_t n) { return n <= 0 ? 16 : ((size_t)n + 15) & ~(size_t)15; }

static void remember_reuse(void *p) {
    unsigned i = reused_n++ % NREUSED;
    reused[i].block = p;
    reused[i].gen = HDR(p)->gen;
    reused[i].freed = HDR(p)->freed;
}

void *ward_alloc(int64_t size) {
    size_t s = round16(size);
    void *p = NULL;
    if (s / 16 <= NCLASS) {
        p = free_small[s / 16];
        if (p) {
            free_small[s / 16] = *(void **)p;
            remember_reuse(p);
        }
    } else {
        for (Big **b = &free_big; *b; b = &(*b)->next) {
            if ((*b)->size == s) {
                Big *x = *b;
                *b = x->next;
                p = x->payload;
                free(x);
                remember_reuse(p);
                break;
            }
        }
    }
    if (!p) {
        WHdr *h = malloc(sizeof(WHdr) + s);
        if (!h) ward_bug("メモリが足りません");
        h->gen = 0;
        h->freed = NULL;
        p = h + 1;
    }
    ward_live++;
    return p;
}

// 区画を解放する（中身の own は書き出したコードが先に解放する）。世代が進むので、ここを指す ref は無効になる
void ward_free(void *p, int64_t size, const WSite *why) {
    WHdr *h = HDR(p);
    h->gen++;
    h->freed = why;
    ward_live--;
    size_t s = round16(size);
    if (s / 16 <= NCLASS) {
        *(void **)p = free_small[s / 16];
        free_small[s / 16] = p;
    } else {
        Big *b = malloc(sizeof(Big));
        b->next = free_big;
        b->size = s;
        b->payload = p;
        free_big = b;
    }
}

uint64_t ward_gen(void *p) { return HDR(p)->gen; }

// ---------------------------------------------------------------- own の読むだけの写し

typedef struct WCell {
    void *target;
    uint64_t gen;
    const WSite *site;
} WCell;

#define IS_VIEW(p) (((uintptr_t)(p)) & 1)
#define CELL(p) ((WCell *)(((uintptr_t)(p)) & ~(uintptr_t)1))

// own（または写し）の写しを作る。写しの写しは同じ先・同じ借りた場所
void *ward_view(void *own, const WSite *site) {
    WCell *c = malloc(sizeof(WCell));
    if (IS_VIEW(own)) {
        *c = *CELL(own);
    } else {
        c->target = own;
        c->gen = HDR(own)->gen;
        c->site = site;
    }
    return (void *)((uintptr_t)c | 1);
}

void ward_view_free(void *v) { free(CELL(v)); }

// 写しの先（世代を確かめる）
void *ward_view_target(void *v, const WSite *use) {
    WCell *c = CELL(v);
    if (HDR(c->target)->gen != c->gen) ward_fail_dangling(use, c->site, c->target, c->gen);
    return c->target;
}

void *ward_view_target_nocheck(void *v) { return CELL(v)->target; }
uint64_t ward_view_gen(void *v) { return CELL(v)->gen; }
const WSite *ward_view_site(void *v) { return CELL(v)->site; }

// ---------------------------------------------------------------- 文字列

WStr *ward_str_new(const char *p, int64_t n) {
    WStr *s = malloc(sizeof(WStr) + (size_t)n + 1);
    s->rc = 1;
    s->len = n;
    memcpy(s->data, p, (size_t)n);
    s->data[n] = 0;
    return s;
}

void ward_str_retain(WStr *s) {
    if (s->rc > 0) s->rc++;
}

void ward_str_release(WStr *s) {
    if (s->rc > 0 && --s->rc == 0) free(s);
}

WStr *ward_str_concat(WStr *a, WStr *b) {
    WStr *s = malloc(sizeof(WStr) + (size_t)(a->len + b->len) + 1);
    s->rc = 1;
    s->len = a->len + b->len;
    memcpy(s->data, a->data, (size_t)a->len);
    memcpy(s->data + a->len, b->data, (size_t)b->len);
    s->data[s->len] = 0;
    return s;
}

// バイトの辞書順（Rust の str の比較と同じ）
int32_t ward_str_cmp(WStr *a, WStr *b) {
    int64_t n = a->len < b->len ? a->len : b->len;
    int c = memcmp(a->data, b->data, (size_t)n);
    if (c) return c < 0 ? -1 : 1;
    return a->len < b->len ? -1 : a->len > b->len ? 1 : 0;
}

int32_t ward_str_eq(WStr *a, WStr *b) { return a->len == b->len && memcmp(a->data, b->data, (size_t)a->len) == 0; }

// ---------------------------------------------------------------- 文字にする

WSb *ward_sb_new(void) {
    WSb *b = malloc(sizeof(WSb));
    b->cap = 64;
    b->len = 0;
    b->p = malloc(64);
    return b;
}

void ward_sb_push(WSb *b, const char *p, int64_t n) {
    if (b->len + n > b->cap) {
        while (b->len + n > b->cap) b->cap *= 2;
        b->p = realloc(b->p, (size_t)b->cap);
    }
    memcpy(b->p + b->len, p, (size_t)n);
    b->len += n;
}

void ward_sb_cstr(WSb *b, const char *s) { ward_sb_push(b, s, (int64_t)strlen(s)); }
void ward_sb_str(WSb *b, WStr *s) { ward_sb_push(b, s->data, s->len); }

void ward_sb_int(WSb *b, int64_t v, int32_t is_unsigned) {
    char buf[32];
    fmt_int(buf, sizeof buf, v, is_unsigned);
    ward_sb_cstr(b, buf);
}

// Rust の `{:?}` と同じ形（いちばん短く元に戻る桁。1e16 以上と 1e-4 未満は指数の形）
static void fmt_f64(char *out, double x) {
    if (isnan(x)) {
        strcpy(out, "NaN");
        return;
    }
    if (isinf(x)) {
        strcpy(out, x < 0 ? "-inf" : "inf");
        return;
    }
    if (x == 0) {
        strcpy(out, signbit(x) ? "-0.0" : "0.0");
        return;
    }
    char tmp[64];
    int p;
    for (p = 1; p <= 17; p++) {
        snprintf(tmp, sizeof tmp, "%.*e", p - 1, x);
        if (strtod(tmp, NULL) == x) break;
    }
    // ちょうど真ん中（printf は偶数へ丸める）なら、Rust と同じく大きいほうを選ぶ
    {
        static char exact[1200];
        snprintf(exact, sizeof exact, "%.*e", p - 1 + 800, x);
        const char *q = exact + (exact[0] == '-');
        int seen = 0, tie = 1;
        const char *r = q;
        for (; *r && *r != 'e'; r++) {
            if (*r < '0' || *r > '9') continue;
            if (seen == p) tie = *r == '5';
            else if (seen > p && *r != '0') tie = 0;
            seen++;
        }
        if (tie && seen > p) {
            // 切り捨てた桁と、丸めた結果の最後の桁が同じなら、下へ丸められている
            char up[64];
            strcpy(up, tmp);
            char *e = strchr(up, 'e');
            char *last = e - 1;
            const char *q2 = q;
            int k = 0;
            char trunc_last = 0;
            for (; *q2 && *q2 != 'e'; q2++) {
                if (*q2 < '0' || *q2 > '9') continue;
                if (k == p - 1) trunc_last = *q2;
                k++;
            }
            if (*last == trunc_last) {
                char *d = last;
                for (;;) {
                    if (*d == '.') {
                        d--;
                        continue;
                    }
                    if (*d == '-' || d < up) break;
                    if (*d == '9') {
                        *d = '0';
                        if (d == up || d[-1] == '-') break;
                        d--;
                        continue;
                    }
                    (*d)++;
                    break;
                }
                if (strtod(up, NULL) == x) strcpy(tmp, up);
            }
        }
    }
    // tmp は [-]d.ddde[+-]xx
    char digits[32];
    int nd = 0;
    const char *q = tmp;
    int neg = 0;
    if (*q == '-') {
        neg = 1;
        q++;
    }
    for (; *q && *q != 'e'; q++)
        if (*q >= '0' && *q <= '9') digits[nd++] = *q;
    int e = atoi(q + 1);
    while (nd > 1 && digits[nd - 1] == '0') nd--;
    char *o = out;
    if (neg) *o++ = '-';
    double a = fabs(x);
    if (a < 1e16 && a >= 1e-4) {
        if (e < 0) {
            *o++ = '0';
            *o++ = '.';
            for (int i = 0; i < -e - 1; i++) *o++ = '0';
            for (int i = 0; i < nd; i++) *o++ = digits[i];
        } else {
            for (int i = 0; i <= e; i++) *o++ = i < nd ? digits[i] : '0';
            *o++ = '.';
            if (nd > e + 1) {
                for (int i = e + 1; i < nd; i++) *o++ = digits[i];
            } else {
                *o++ = '0';
            }
        }
        *o = 0;
    } else {
        *o++ = digits[0];
        if (nd > 1) {
            *o++ = '.';
            for (int i = 1; i < nd; i++) *o++ = digits[i];
        }
        sprintf(o, "e%d", e);
    }
}

void ward_sb_float(WSb *b, double x) {
    char buf[64];
    fmt_f64(buf, x);
    ward_sb_cstr(b, buf);
}

WStr *ward_sb_finish(WSb *b) {
    WStr *s = ward_str_new(b->p, b->len);
    free(b->p);
    free(b);
    return s;
}

// ---------------------------------------------------------------- 数

// 小数 → 整数（Rust の `trunc().clamp(i128) as i128` を型の幅で一周させたもの）
int64_t ward_f2i(double x, int32_t bits, int32_t is_signed) {
    uint64_t u;
    if (isnan(x)) return 0;
    double t = trunc(x);
    if (t >= 0x1p127) {
        u = ~(uint64_t)0;
    } else if (t <= -0x1p127) {
        u = 0;
    } else if (t > -0x1p63 && t < 0x1p63) {
        u = (uint64_t)(int64_t)t;
    } else {
        double r = fmod(t, 0x1p64);
        if (r < 0) r += 0x1p64;
        u = (uint64_t)r;
    }
    if (bits < 64) {
        uint64_t m = ((uint64_t)1 << bits) - 1;
        u &= m;
        if (is_signed && (u >> (bits - 1)) & 1) u |= ~m;
    }
    return (int64_t)u;
}

// ---------------------------------------------------------------- リスト

void ward_list_reserve(WList *l, int64_t esize, int64_t n) {
    if (n <= l->cap) return;
    int64_t cap = l->cap ? l->cap : 4;
    while (cap < n) cap *= 2;
    l->data = realloc(l->data, (size_t)(cap * (esize ? esize : 1)));
    l->cap = cap;
}

// 終わりに 1 つ場所を足して、その場所を返す
char *ward_list_push_slot(WList *l, int64_t esize) {
    ward_list_reserve(l, esize, l->len + 1);
    return l->data + (l->len++) * esize;
}

// i 番目に場所を空けて、その場所を返す
char *ward_list_insert_slot(WList *l, int64_t esize, int64_t i) {
    ward_list_reserve(l, esize, l->len + 1);
    memmove(l->data + (i + 1) * esize, l->data + i * esize, (size_t)((l->len - i) * esize));
    l->len++;
    return l->data + i * esize;
}

// i 番目を out へ移して詰める
void ward_list_remove(WList *l, int64_t esize, int64_t i, void *out) {
    memcpy(out, l->data + i * esize, (size_t)esize);
    memmove(l->data + i * esize, l->data + (i + 1) * esize, (size_t)((l->len - i - 1) * esize));
    l->len--;
}

void ward_list_free(WList *l) { free(l->data); }

// 並べ替え（安定）。less(a, b, env) で比べる
typedef int32_t (*WLess)(void *a, void *b, void *env);

static void msort(char *a, char *tmp, int64_t n, int64_t es, WLess less, void *env) {
    if (n < 2) return;
    int64_t h = n / 2;
    msort(a, tmp, h, es, less, env);
    msort(a + h * es, tmp, n - h, es, less, env);
    int64_t i = 0, j = h, k = 0;
    while (i < h && j < n) {
        if (less(a + j * es, a + i * es, env)) memcpy(tmp + (k++) * es, a + (j++) * es, (size_t)es);
        else memcpy(tmp + (k++) * es, a + (i++) * es, (size_t)es);
    }
    while (i < h) memcpy(tmp + (k++) * es, a + (i++) * es, (size_t)es);
    while (j < n) memcpy(tmp + (k++) * es, a + (j++) * es, (size_t)es);
    memcpy(a, tmp, (size_t)(n * es));
}

void ward_sort(char *data, int64_t n, int64_t es, WLess less, void *env) {
    if (n < 2) return;
    char *tmp = malloc((size_t)(n * es));
    msort(data, tmp, n, es, less, env);
    free(tmp);
}

// ---------------------------------------------------------------- 辞書（入れた順を保つ。キーは 8 バイト: 整数・真偽値・文字列のポインタ）

typedef struct WMap {
    int64_t len, cap, esize, icap;
    char *e;
    int64_t *idx; // 位置 + 1（0 は空き）
    int32_t kind; // 0 整数・1 文字列・2 真偽値
} WMap;

static uint64_t key_hash(WMap *m, int64_t k) {
    if (m->kind == 1) {
        WStr *s = (WStr *)(intptr_t)k;
        uint64_t h = 1469598103934665603ULL;
        for (int64_t i = 0; i < s->len; i++) h = (h ^ (unsigned char)s->data[i]) * 1099511628211ULL;
        return h;
    }
    uint64_t z = (uint64_t)k + 0x9e3779b97f4a7c15ULL;
    z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL;
    z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL;
    return z ^ (z >> 31);
}

static int key_eq(WMap *m, int64_t a, int64_t b) {
    if (m->kind == 1) return ward_str_eq((WStr *)(intptr_t)a, (WStr *)(intptr_t)b);
    return a == b;
}

static int64_t map_key(WMap *m, int64_t i) { return *(int64_t *)(m->e + i * m->esize); }

static void index_put(WMap *m, int64_t i) {
    uint64_t mask = (uint64_t)m->icap - 1;
    uint64_t h = key_hash(m, map_key(m, i)) & mask;
    while (m->idx[h]) h = (h + 1) & mask;
    m->idx[h] = i + 1;
}

static void reindex(WMap *m) {
    int64_t cap = 8;
    while (cap < m->len * 2 + 2) cap *= 2;
    free(m->idx);
    m->idx = calloc((size_t)cap, sizeof(int64_t));
    m->icap = cap;
    for (int64_t i = 0; i < m->len; i++) index_put(m, i);
}

WMap *ward_map_new(int64_t vsize, int32_t kind) {
    WMap *m = calloc(1, sizeof(WMap));
    m->esize = 8 + ((vsize + 7) & ~(int64_t)7);
    m->kind = kind;
    return m;
}

int64_t ward_map_find(WMap *m, int64_t key) {
    if (!m->icap) return -1;
    uint64_t mask = (uint64_t)m->icap - 1;
    uint64_t h = key_hash(m, key) & mask;
    while (m->idx[h]) {
        int64_t i = m->idx[h] - 1;
        if (key_eq(m, map_key(m, i), key)) return i;
        h = (h + 1) & mask;
    }
    return -1;
}

// キーを終わりに足して、その位置を返す（値の場所は書き出したコードが埋める）
int64_t ward_map_add(WMap *m, int64_t key) {
    if (m->len == m->cap) {
        m->cap = m->cap ? m->cap * 2 : 4;
        m->e = realloc(m->e, (size_t)(m->cap * m->esize));
    }
    int64_t i = m->len++;
    *(int64_t *)(m->e + i * m->esize) = key;
    if (m->len * 2 + 2 > m->icap) reindex(m);
    else index_put(m, i);
    return i;
}

char *ward_map_val(WMap *m, int64_t i) { return m->e + i * m->esize + 8; }
int64_t ward_map_key(WMap *m, int64_t i) { return map_key(m, i); }
int64_t ward_map_len(WMap *m) { return m->len; }

// i 番目を取り除いて後ろを詰める（キーと値の解放は書き出したコードがする）
void ward_map_remove_at(WMap *m, int64_t i) {
    memmove(m->e + i * m->esize, m->e + (i + 1) * m->esize, (size_t)((m->len - i - 1) * m->esize));
    m->len--;
    reindex(m);
}

void ward_map_free(WMap *m) {
    free(m->e);
    free(m->idx);
    free(m);
}

// ---------------------------------------------------------------- 文字列のメソッド（位置と長さは文字単位）

static int is_cont(unsigned char c) { return (c & 0xC0) == 0x80; }

int64_t ward_str_chars(WStr *s) {
    int64_t n = 0;
    for (int64_t i = 0; i < s->len; i++) n += !is_cont((unsigned char)s->data[i]);
    return n;
}

static void push_str(WList *out, WStr *x) { *(WStr **)ward_list_push_slot(out, 8) = x; }

void ward_str_chars_list(WStr *s, WList *out) {
    int64_t i = 0;
    while (i < s->len) {
        int64_t j = i + 1;
        while (j < s->len && is_cont((unsigned char)s->data[j])) j++;
        push_str(out, ward_str_new(s->data + i, j - i));
        i = j;
    }
}

// Rust の `lines()`: \n で分け、行末の \r を除く。最後の \n のあとの空の行は数えない
void ward_str_lines(WStr *s, WList *out) {
    int64_t i = 0;
    while (i < s->len) {
        int64_t j = i;
        while (j < s->len && s->data[j] != '\n') j++;
        int64_t end = j;
        if (end > i && s->data[end - 1] == '\r') end--;
        push_str(out, ward_str_new(s->data + i, end - i));
        i = j + 1;
    }
}

static int64_t find_from(WStr *s, WStr *pat, int64_t from) {
    if (pat->len == 0) return from;
    for (int64_t i = from; i + pat->len <= s->len; i++)
        if (memcmp(s->data + i, pat->data, (size_t)pat->len) == 0) return i;
    return -1;
}

void ward_str_split(WStr *s, WStr *sep, WList *out) {
    int64_t i = 0;
    for (;;) {
        int64_t j = find_from(s, sep, i);
        if (j < 0) break;
        push_str(out, ward_str_new(s->data + i, j - i));
        i = j + sep->len;
    }
    push_str(out, ward_str_new(s->data + i, s->len - i));
}

// UTF-8 の 1 文字を読む（*len に長さ）
static uint32_t decode(const unsigned char *p, int64_t n, int64_t *len) {
    if (p[0] < 0x80 || n < 2) {
        *len = 1;
        return p[0];
    }
    if ((p[0] & 0xE0) == 0xC0) {
        *len = 2;
        return ((uint32_t)(p[0] & 0x1F) << 6) | (p[1] & 0x3F);
    }
    if ((p[0] & 0xF0) == 0xE0 && n >= 3) {
        *len = 3;
        return ((uint32_t)(p[0] & 0x0F) << 12) | ((uint32_t)(p[1] & 0x3F) << 6) | (p[2] & 0x3F);
    }
    if (n >= 4) {
        *len = 4;
        return ((uint32_t)(p[0] & 0x07) << 18) | ((uint32_t)(p[1] & 0x3F) << 12) | ((uint32_t)(p[2] & 0x3F) << 6) | (p[3] & 0x3F);
    }
    *len = 1;
    return p[0];
}

// Unicode の White_Space（Rust の `trim` と同じ）
static int is_space(uint32_t c) {
    return (c >= 9 && c <= 13) || c == 32 || c == 0x85 || c == 0xA0 || c == 0x1680 || (c >= 0x2000 && c <= 0x200A) ||
           c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F || c == 0x3000;
}

static void trim_range(WStr *s, int64_t *a, int64_t *b) {
    const unsigned char *p = (const unsigned char *)s->data;
    int64_t i = 0, j = s->len;
    while (i < j) {
        int64_t l;
        uint32_t c = decode(p + i, j - i, &l);
        if (!is_space(c)) break;
        i += l;
    }
    while (j > i) {
        int64_t k = j - 1;
        while (k > i && is_cont(p[k])) k--;
        int64_t l;
        uint32_t c = decode(p + k, j - k, &l);
        if (!is_space(c)) break;
        j = k;
    }
    *a = i;
    *b = j;
}

WStr *ward_str_trim(WStr *s) {
    int64_t a, b;
    trim_range(s, &a, &b);
    return ward_str_new(s->data + a, b - a);
}

WStr *ward_str_upper(WStr *s) {
    WStr *r = ward_str_new(s->data, s->len);
    for (int64_t i = 0; i < r->len; i++)
        if (r->data[i] >= 'a' && r->data[i] <= 'z') r->data[i] -= 32;
    return r;
}

WStr *ward_str_lower(WStr *s) {
    WStr *r = ward_str_new(s->data, s->len);
    for (int64_t i = 0; i < r->len; i++)
        if (r->data[i] >= 'A' && r->data[i] <= 'Z') r->data[i] += 32;
    return r;
}

int32_t ward_str_contains(WStr *s, WStr *p) { return find_from(s, p, 0) >= 0; }
int32_t ward_str_starts(WStr *s, WStr *p) { return p->len <= s->len && memcmp(s->data, p->data, (size_t)p->len) == 0; }
int32_t ward_str_ends(WStr *s, WStr *p) {
    return p->len <= s->len && memcmp(s->data + s->len - p->len, p->data, (size_t)p->len) == 0;
}

// 見つかった位置（文字単位）。なければ -1
int64_t ward_str_find(WStr *s, WStr *p) {
    int64_t b = find_from(s, p, 0);
    if (b < 0) return -1;
    int64_t n = 0;
    for (int64_t i = 0; i < b; i++) n += !is_cont((unsigned char)s->data[i]);
    return n;
}

// 文字の位置 → バイトの位置
static int64_t byte_at(WStr *s, int64_t ch) {
    int64_t i = 0, n = 0;
    while (i < s->len) {
        if (!is_cont((unsigned char)s->data[i])) {
            if (n == ch) return i;
            n++;
        }
        i++;
    }
    return s->len;
}

WStr *ward_str_slice(WStr *s, int64_t a, int64_t b, const WSite *at) {
    int64_t n = ward_str_chars(s);
    if (a < 0 || b < a || b > n) {
        char label[256];
        snprintf(label, sizeof label, "slice(%lld, %lld) ですが、長さは %lld です", (long long)a, (long long)b, (long long)n);
        begin_error("文字列の範囲の外です");
        finish_error(at, label, "0 <= 始め <= 終わり <= 長さ にしてください");
    }
    int64_t x = byte_at(s, a), y = byte_at(s, b);
    return ward_str_new(s->data + x, y - x);
}

WStr *ward_str_replace(WStr *s, WStr *from, WStr *to) {
    WSb *b = ward_sb_new();
    if (from->len == 0) {
        // Rust と同じく、文字の境目すべてに入れる
        int64_t i = 0;
        ward_sb_str(b, to);
        while (i < s->len) {
            int64_t j = i + 1;
            while (j < s->len && is_cont((unsigned char)s->data[j])) j++;
            ward_sb_push(b, s->data + i, j - i);
            ward_sb_str(b, to);
            i = j;
        }
        return ward_sb_finish(b);
    }
    int64_t i = 0;
    for (;;) {
        int64_t j = find_from(s, from, i);
        if (j < 0) break;
        ward_sb_push(b, s->data + i, j - i);
        ward_sb_str(b, to);
        i = j + from->len;
    }
    ward_sb_push(b, s->data + i, s->len - i);
    return ward_sb_finish(b);
}

WStr *ward_str_repeat(WStr *s, int64_t k, const WSite *at) {
    if (k < 0) {
        char label[64];
        snprintf(label, sizeof label, "repeat(%lld)", (long long)k);
        begin_error("負の回数はくり返せません");
        finish_error(at, label, NULL);
    }
    WSb *b = ward_sb_new();
    for (int64_t i = 0; i < k; i++) ward_sb_str(b, s);
    return ward_sb_finish(b);
}

// Rust の `trim().parse::<i64>()`
int32_t ward_str_to_int(WStr *s, int64_t *out) {
    int64_t a, b;
    trim_range(s, &a, &b);
    const char *p = s->data + a;
    int64_t n = b - a;
    int neg = 0;
    if (n > 0 && (p[0] == '+' || p[0] == '-')) {
        neg = p[0] == '-';
        p++;
        n--;
    }
    if (n == 0) return 0;
    uint64_t v = 0;
    for (int64_t i = 0; i < n; i++) {
        if (p[i] < '0' || p[i] > '9') return 0;
        uint64_t d = (uint64_t)(p[i] - '0');
        if (v > (UINT64_MAX - d) / 10) return 0;
        v = v * 10 + d;
    }
    if (neg) {
        if (v > (uint64_t)INT64_MAX + 1) return 0;
        *out = (int64_t)(0 - v);
    } else {
        if (v > (uint64_t)INT64_MAX) return 0;
        *out = (int64_t)v;
    }
    return 1;
}

// order の順に要素を並べ直す（sort_by）
void ward_permute(char *data, int64_t n, int64_t es, const int64_t *order) {
    if (n < 2) return;
    char *tmp = malloc((size_t)(n * es));
    for (int64_t k = 0; k < n; k++) memcpy(tmp + k * es, data + order[k] * es, (size_t)es);
    memcpy(data, tmp, (size_t)(n * es));
    free(tmp);
}

// ---------------------------------------------------------------- 関数の値（取り込んだ値の入れ物）
//
// 頭 24 バイト: 数・表（解放と写しの関数）・印（1: ムーブする・2: own を持つ・4: 元の値がムーブするものだった）。
// そのあとに取り込んだ値が並ぶ。取り込んだ値は書き換えないので、複製は入れ物を共有して数を増やす。

typedef struct WEnvVt {
    void (*drop)(void *env, const WSite *site);
    void *(*view)(void *env, const WSite *site);
} WEnvVt;

typedef struct WEnv {
    int64_t rc;
    const WEnvVt *vt;
    int64_t flags;
} WEnv;

void *ward_env_new(int64_t size, const WEnvVt *vt) {
    WEnv *e = calloc(1, (size_t)size);
    e->rc = 1;
    e->vt = vt;
    return e;
}

void ward_env_retain(WEnv *e) { e->rc++; }

void ward_env_release(WEnv *e, const WSite *site) {
    if (--e->rc == 0) {
        e->vt->drop(e, site);
        free(e);
    }
}

// ---------------------------------------------------------------- 入力

static int g_argc;
static char **g_argv;

// 1 行読む（終わりの改行は取る）。入力が終わっていれば 0
int32_t ward_read_line(WStr **out) {
    ward_flush();
    WSb *b = ward_sb_new();
    int c, any = 0;
    while ((c = getchar()) != EOF) {
        any = 1;
        if (c == '\n') break;
        char ch = (char)c;
        ward_sb_push(b, &ch, 1);
    }
    if (!any) {
        free(b->p);
        free(b);
        return 0;
    }
    if (b->len > 0 && b->p[b->len - 1] == '\r') b->len--;
    *out = ward_sb_finish(b);
    return 1;
}

// コマンドライン引数（プログラムの名前は除く）
void ward_args(WList *out) {
    for (int i = 1; i < g_argc; i++) push_str(out, ward_str_new(g_argv[i], (int64_t)strlen(g_argv[i])));
}

// ---------------------------------------------------------------- 入口

void ward_main(void);

#ifdef _WIN32
// Windows: 端末の文字コードを UTF-8 にし、引数は UTF-16 から UTF-8 に直す（windows.h なしで宣言する）
#include <wchar.h>
__declspec(dllimport) int __stdcall SetConsoleOutputCP(unsigned);
__declspec(dllimport) int __stdcall SetConsoleCP(unsigned);
__declspec(dllimport) wchar_t *__stdcall GetCommandLineW(void);
__declspec(dllimport) wchar_t **__stdcall CommandLineToArgvW(const wchar_t *, int *);
__declspec(dllimport) int __stdcall WideCharToMultiByte(unsigned, unsigned long, const wchar_t *, int, char *, int, const char *, int *);

static void win_setup(int *argc, char ***argv) {
    SetConsoleOutputCP(65001);
    SetConsoleCP(65001);
    int n = 0;
    wchar_t **w = CommandLineToArgvW(GetCommandLineW(), &n);
    if (!w) return;
    char **a = malloc(sizeof(char *) * (size_t)(n + 1));
    for (int i = 0; i < n; i++) {
        int len = WideCharToMultiByte(65001, 0, w[i], -1, NULL, 0, NULL, NULL);
        a[i] = malloc((size_t)len);
        WideCharToMultiByte(65001, 0, w[i], -1, a[i], len, NULL, NULL);
    }
    a[n] = NULL;
    *argc = n;
    *argv = a;
}
#endif

int main(int argc, char **argv) {
#ifdef _WIN32
    win_setup(&argc, &argv);
#endif
    g_argc = argc;
    g_argv = argv;
    ward_main();
    ward_flush();
    if (getenv("WARD_LEAKS")) fprintf(stderr, "[live] %lld\n", (long long)ward_live);
    return 0;
}
