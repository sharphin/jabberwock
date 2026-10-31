/* jwvault: パスワードの金庫の窓。エンジン（jw_engine.h）を動かし、jabberwock で書いた金庫の本体
 * （browser/app/vault_main.jw。実行ファイルは jwvault-app）を子プロセスとして起動して、標準入出力の 1 行ずつでつなぐ。
 *
 * ブラウザ本体はもう std.browser でエンジンを直接使う。金庫は、クリップボード・ブラウザとのやりとりがまだ std にないので、
 * そのあいだのつなぎとしてこれを使う。行の形は README.md。どのエンジンともリンクできる（WebView2・偽エンジン）。
 *
 * 金庫はページを開かず（タブを作らない）、WebView2 の置き場所もブラウザと分ける（<置き場所>/vault）。ブラウザとは
 * Windows のメッセージ（WM_COPYDATA）だけで話す: 金庫 → ブラウザ「fill サイト ユーザー パスワード」「site?」、
 * ブラウザ → 金庫「site URL」「filled 結果」「show」。ブラウザ側は engine/webview2/vault_link.c。
 * ブラウザは今のタブの出どころがサイトと完全に同じときだけ入れる（jwe_fill_login）。
 *
 * 偽エンジン（名前が "mock"）のときは、本体が "wait" を出す（次のイベントを待ち始めた）まで次のイベントを渡さない。
 * そうすると、何回動かしても同じ順になる（テスト用）。
 */
#include "../../runtime/engine/jw_engine.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
typedef HANDLE Fd;
#else
#include <fcntl.h>
#include <time.h>
#include <pthread.h>
#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>
typedef int Fd;
#endif

/* ---- 文字 ---- */

typedef struct {
    char *p;
    size_t len, cap;
} Buf;

static void buf_put(Buf *b, const char *s, size_t n) {
    if (b->len + n + 1 > b->cap) {
        size_t c = b->cap ? b->cap : 256;
        while (c < b->len + n + 1) c *= 2;
        b->p = (char *)realloc(b->p, c);
        if (!b->p) abort();
        b->cap = c;
    }
    memcpy(b->p + b->len, s, n);
    b->len += n;
    b->p[b->len] = 0;
}

static void buf_str(Buf *b, const char *s) { buf_put(b, s, strlen(s)); }

/* \ タブ 改行 を \\ \t \n に。復帰は捨てる */
static void buf_escaped(Buf *b, const char *s, size_t n) {
    for (size_t i = 0; i < n; i++) {
        char c = s[i];
        if (c == '\\') buf_put(b, "\\\\", 2);
        else if (c == '\t') buf_put(b, "\\t", 2);
        else if (c == '\n') buf_put(b, "\\n", 2);
        else if (c == '\r' || c == 0x1a) continue; /* 0x1a は Windows の入力で終わりの印になる */
        else buf_put(b, &c, 1);
    }
}

/* その場で戻す。長さを返す */
static size_t unescape(char *s) {
    char *w = s;
    for (char *r = s; *r; r++) {
        if (*r == '\\' && r[1]) {
            r++;
            *w++ = *r == 't' ? '\t' : *r == 'n' ? '\n' : *r;
        } else {
            *w++ = *r;
        }
    }
    *w = 0;
    return (size_t)(w - s);
}

/* ---- 本体から届いた行の列（読むスレッド → 主スレッド） ---- */

typedef struct Line {
    struct Line *next;
    char text[];
} Line;

static Line *q_head, *q_tail;
static int q_waits;   /* 本体が "wait" を出した回数のうち、まだ使っていないもの */
static int q_eof;
#ifdef _WIN32
static CRITICAL_SECTION q_lock;
static HANDLE q_signal;
static DWORD main_thread;
#define LOCK() EnterCriticalSection(&q_lock)
#define UNLOCK() LeaveCriticalSection(&q_lock)
#else
static pthread_mutex_t q_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t q_cond = PTHREAD_COND_INITIALIZER;
#define LOCK() pthread_mutex_lock(&q_lock)
#define UNLOCK() pthread_mutex_unlock(&q_lock)
#endif

static void q_wake(void) {
#ifdef _WIN32
    SetEvent(q_signal);
    PostThreadMessageW(main_thread, WM_NULL, 0, 0); /* jwe_wait の待ちを起こす */
#else
    pthread_cond_broadcast(&q_cond);
#endif
}

static void q_push(const char *s, size_t n) {
    if (n == 4 && memcmp(s, "wait", 4) == 0) {
        LOCK();
        q_waits++;
        q_wake();
        UNLOCK();
        return;
    }
    Line *l = (Line *)malloc(sizeof(Line) + n + 1);
    if (!l) abort();
    memcpy(l->text, s, n);
    l->text[n] = 0;
    l->next = NULL;
    LOCK();
    if (q_tail) q_tail->next = l;
    else q_head = l;
    q_tail = l;
    q_wake();
    UNLOCK();
}

static Line *q_pop(void) {
    LOCK();
    Line *l = q_head;
    if (l) {
        q_head = l->next;
        if (!q_head) q_tail = NULL;
    }
    UNLOCK();
    return l;
}

static Fd from_app, to_app;

static long fd_read(Fd f, char *p, size_t n) {
#ifdef _WIN32
    DWORD got = 0;
    if (!ReadFile(f, p, (DWORD)n, &got, NULL)) return 0;
    return (long)got;
#else
    return (long)read(f, p, n);
#endif
}

static int fd_write(Fd f, const char *p, size_t n) {
    while (n) {
#ifdef _WIN32
        DWORD put = 0;
        if (!WriteFile(f, p, (DWORD)n, &put, NULL)) return 0;
#else
        long put = (long)write(f, p, n);
        if (put <= 0) return 0;
#endif
        p += put;
        n -= (size_t)put;
    }
    return 1;
}

#ifdef _WIN32
static DWORD WINAPI reader(LPVOID arg)
#else
static void *reader(void *arg)
#endif
{
    (void)arg;
    Buf line = {0};
    char chunk[4096];
    long n;
    while ((n = fd_read(from_app, chunk, sizeof chunk)) > 0) {
        for (long i = 0; i < n; i++) {
            if (chunk[i] == '\n') {
                size_t len = line.len;
                if (len && line.p[len - 1] == '\r') len--;
                q_push(line.p ? line.p : "", len);
                line.len = 0;
            } else {
                buf_put(&line, &chunk[i], 1);
            }
        }
    }
    LOCK();
    q_eof = 1;
    q_wake();
    UNLOCK();
    free(line.p);
    return 0;
}

/* ---- 本体の起動 ---- */

static void fail(const char *msg) {
    fprintf(stderr, "jwvault: %s\n", msg);
#ifdef _WIN32
    int n = MultiByteToWideChar(CP_UTF8, 0, msg, -1, NULL, 0);
    wchar_t *w = (wchar_t *)malloc(sizeof(wchar_t) * (size_t)n);
    MultiByteToWideChar(CP_UTF8, 0, msg, -1, w, n);
    MessageBoxW(NULL, w, L"jabberwock", MB_OK | MB_ICONERROR);
    free(w);
#endif
}

#ifdef _WIN32
static HANDLE app_process;

/* 実行ファイルと同じフォルダの name の場所 */
static void beside_exe(wchar_t *out, size_t cap, const wchar_t *name) {
    GetModuleFileNameW(NULL, out, (DWORD)cap);
    wchar_t *slash = wcsrchr(out, L'\\');
    if (slash) slash[1] = 0;
    else out[0] = 0;
    wcsncat(out, name, cap - wcslen(out) - 1);
}

static int start_app(const char *data_dir) {
    wchar_t exe[MAX_PATH];
    beside_exe(exe, MAX_PATH, L"jwvault-app.exe");
    if (GetFileAttributesW(exe) == INVALID_FILE_ATTRIBUTES) {
        fail("jwvault-app.exe が見つかりません（jwvault.exe と同じフォルダに置いてください）");
        return 0;
    }
    wchar_t dir[MAX_PATH];
    MultiByteToWideChar(CP_UTF8, 0, data_dir, -1, dir, MAX_PATH);
    wchar_t cmd[3 * MAX_PATH];
    _snwprintf(cmd, 3 * MAX_PATH, L"\"%ls\" --data \"%ls\"", exe, dir);
    cmd[3 * MAX_PATH - 1] = 0;

    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};
    HANDLE in_r, in_w, out_r, out_w;
    if (!CreatePipe(&in_r, &in_w, &sa, 0) || !CreatePipe(&out_r, &out_w, &sa, 0)) return 0;
    SetHandleInformation(in_w, HANDLE_FLAG_INHERIT, 0); /* 子に渡すのは片側だけ */
    SetHandleInformation(out_r, HANDLE_FLAG_INHERIT, 0);
    STARTUPINFOW si;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = in_r;
    si.hStdOutput = out_w;
    si.hStdError = GetStdHandle(STD_ERROR_HANDLE);
    PROCESS_INFORMATION pi;
    if (!CreateProcessW(exe, cmd, NULL, NULL, TRUE, CREATE_NO_WINDOW, NULL, NULL, &si, &pi)) {
        fail("金庫の本体（jwvault-app.exe）を起動できません");
        return 0;
    }
    CloseHandle(pi.hThread);
    CloseHandle(in_r);
    CloseHandle(out_w);
    app_process = pi.hProcess;
    to_app = in_w;
    from_app = out_r;
    InitializeCriticalSection(&q_lock);
    q_signal = CreateEventW(NULL, FALSE, FALSE, NULL);
    main_thread = GetCurrentThreadId();
    HANDLE t = CreateThread(NULL, 0, reader, NULL, 0, NULL);
    if (t) CloseHandle(t);
    return t != NULL;
}

static void stop_app(void) {
    CloseHandle(to_app);
    if (WaitForSingleObject(app_process, 3000) == WAIT_TIMEOUT) TerminateProcess(app_process, 1);
    CloseHandle(app_process);
}
#else
static pid_t app_pid;

static int start_app(const char *data_dir) {
    /* 例: "jabberwock main.jw" は使えないので、実行ファイルを指す */
    const char *exe = getenv("JWVAULT_APP");
    if (!exe || !*exe) exe = "./jwvault-app";
    int in[2], out[2];
    if (pipe(in) || pipe(out)) return 0;
    app_pid = fork();
    if (app_pid < 0) return 0;
    if (app_pid == 0) {
        dup2(in[0], 0);
        dup2(out[1], 1);
        close(in[0]); close(in[1]); close(out[0]); close(out[1]);
        execl(exe, exe, "--data", data_dir, (char *)NULL);
        _exit(127);
    }
    close(in[0]);
    close(out[1]);
    to_app = in[1];
    from_app = out[0];
    signal(SIGPIPE, SIG_IGN);
    pthread_t t;
    return pthread_create(&t, NULL, reader, NULL) == 0 && pthread_detach(t) == 0;
}

static void stop_app(void) {
    close(to_app);
    int st;
    waitpid(app_pid, &st, 0);
}
#endif

/* ---- 金庫とブラウザのあいだ ---- */

static char *data_arg; /* --data で渡されたもの */

static void send_app(const char *s) { fd_write(to_app, s, strlen(s)); }

#ifdef _WIN32
/* 本体へ「kind<タブ>文字」の 1 行 */
static void send_app_text(const char *kind, const char *text, size_t n) {
    Buf b = {0};
    buf_str(&b, kind);
    buf_str(&b, "\t");
    buf_escaped(&b, text, n);
    buf_str(&b, "\n");
    fd_write(to_app, b.p, b.len);
    free(b.p);
}
#endif

static unsigned long long now_ms(void) {
#ifdef _WIN32
    return (unsigned long long)GetTickCount64();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (unsigned long long)ts.tv_sec * 1000ull + (unsigned long long)ts.tv_nsec / 1000000ull;
#endif
}

#define CLIP_CLEAR_MS 30000ull     /* コピーしたパスワードを消すまで */
#define VAULT_IDLE_MS 300000ull    /* 金庫を何もしないで閉じるまで（5 分） */
static unsigned long long clip_at;  /* コピーした時刻（0 はなし） */
static unsigned long long last_use; /* 金庫を最後に触った時刻 */
#ifdef _WIN32
static DWORD clip_seq;
#endif

/* クリップボードに文字を置く（クリップボードの履歴・クラウドには残さない）。30 秒たって変わっていなければ消す */
static void clip_copy(const char *s, size_t n) {
#ifdef _WIN32
    int w = MultiByteToWideChar(CP_UTF8, 0, s, (int)n, NULL, 0);
    HGLOBAL g = GlobalAlloc(GMEM_MOVEABLE, sizeof(wchar_t) * (size_t)(w + 1));
    if (!g) return;
    wchar_t *p = (wchar_t *)GlobalLock(g);
    MultiByteToWideChar(CP_UTF8, 0, s, (int)n, p, w);
    p[w] = 0;
    GlobalUnlock(g);
    if (!OpenClipboard(NULL)) {
        GlobalFree(g);
        return;
    }
    EmptyClipboard();
    SetClipboardData(CF_UNICODETEXT, g);
    HGLOBAL ex = GlobalAlloc(GMEM_MOVEABLE, 4);
    if (ex) SetClipboardData(RegisterClipboardFormatW(L"ExcludeClipboardContentFromMonitorProcessing"), ex);
    const wchar_t *no[] = {L"CanIncludeInClipboardHistory", L"CanUploadToCloudClipboard"};
    for (int i = 0; i < 2; i++) {
        HGLOBAL z = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, sizeof(DWORD));
        if (z) SetClipboardData(RegisterClipboardFormatW(no[i]), z);
    }
    CloseClipboard();
    clip_seq = GetClipboardSequenceNumber();
#else
    (void)s;
    fprintf(stderr, "jwvault: クリップボードへ %u バイト（この OS ではまだ置かない）\n", (unsigned)n);
#endif
    clip_at = now_ms();
    if (!clip_at) clip_at = 1;
}

static void clip_tick(void) {
    if (!clip_at || now_ms() - clip_at < CLIP_CLEAR_MS) return;
    clip_at = 0;
#ifdef _WIN32
    if (GetClipboardSequenceNumber() == clip_seq && OpenClipboard(NULL)) {
        EmptyClipboard();
        CloseClipboard();
    }
#endif
}

/* ほかの窓（ブラウザ ↔ 金庫）へ 1 行送る。相手がいなければ 0 */
#ifdef _WIN32
static const wchar_t IPC_BROWSER[] = L"JabberwockBrowserIpc";
static const wchar_t IPC_VAULT[] = L"JabberwockVaultIpc";
static HWND ipc_hwnd;
static Line *ipc_head, *ipc_tail; /* 届いた行（主スレッドだけで触る） */

static int ipc_send(const wchar_t *to, const char *s, size_t n) {
    HWND h = FindWindowExW(HWND_MESSAGE, NULL, to, NULL);
    if (!h) return 0;
    COPYDATASTRUCT cd;
    cd.dwData = 0x6a77; /* "jw" */
    cd.cbData = (DWORD)n;
    cd.lpData = (void *)s;
    DWORD_PTR r = 0;
    return SendMessageTimeoutW(h, WM_COPYDATA, (WPARAM)ipc_hwnd, (LPARAM)&cd, SMTO_ABORTIFHUNG, 2000, &r) != 0;
}

static LRESULT CALLBACK ipc_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    if (msg == WM_COPYDATA) {
        const COPYDATASTRUCT *cd = (const COPYDATASTRUCT *)lp;
        if (cd->dwData != 0x6a77 || cd->cbData > 65536) return FALSE;
        Line *l = (Line *)malloc(sizeof(Line) + cd->cbData + 1);
        if (!l) return FALSE;
        memcpy(l->text, cd->lpData, cd->cbData);
        l->text[cd->cbData] = 0;
        l->next = NULL;
        if (ipc_tail) ipc_tail->next = l;
        else ipc_head = l;
        ipc_tail = l;
        return TRUE;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static void ipc_open(void) {
    WNDCLASSEXW wc;
    memset(&wc, 0, sizeof wc);
    wc.cbSize = sizeof wc;
    wc.lpfnWndProc = ipc_proc;
    wc.hInstance = GetModuleHandleW(NULL);
    wc.lpszClassName = IPC_VAULT;
    RegisterClassExW(&wc);
    ipc_hwnd = CreateWindowExW(0, wc.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, wc.hInstance, NULL);
}

static Line *ipc_pop(void) {
    Line *l = ipc_head;
    if (l) {
        ipc_head = l->next;
        if (!ipc_head) ipc_tail = NULL;
    }
    return l;
}
#endif

static int to_browser(const char *s) {
#ifdef _WIN32
    return ipc_send(IPC_BROWSER, s, strlen(s));
#else
    (void)s;
    return 0;
#endif
}

/* ---- イベント → 本体 ---- */

static const char *KIND[] = {"none", "ready", "close", "nav_started", "nav_finished", "url", "title",
                             "history", "new_tab", "chrome", "key", "permission", "error"};

static int forward_event(const JweEvent *ev) {
    if (ev->kind == JWE_EV_NONE || ev->kind >= sizeof KIND / sizeof *KIND) return 1;
    if (ev->kind == JWE_EV_CHROME_MESSAGE) last_use = now_ms();
    Buf b = {0};
    char num[32];
    buf_str(&b, KIND[ev->kind]);
    switch (ev->kind) {
    case JWE_EV_READY:
    case JWE_EV_CLOSE_REQUESTED:
        break;
    case JWE_EV_CHROME_MESSAGE:
        buf_str(&b, "\t");
        buf_escaped(&b, ev->text, ev->text_len);
        break;
    case JWE_EV_KEY:
        snprintf(num, sizeof num, "\t%lld", (long long)ev->code);
        buf_str(&b, num);
        break;
    case JWE_EV_ERROR:
        snprintf(num, sizeof num, "\t%lld\t", (long long)ev->code);
        buf_str(&b, num);
        buf_escaped(&b, ev->text, ev->text_len);
        break;
    case JWE_EV_NAV_FINISHED:
    case JWE_EV_HISTORY_CHANGED:
        snprintf(num, sizeof num, "\t%u\t%lld", ev->tab, (long long)ev->code);
        buf_str(&b, num);
        if (ev->kind == JWE_EV_NAV_FINISHED) {
            buf_str(&b, "\t");
            buf_escaped(&b, ev->text, ev->text_len);
        }
        break;
    default: /* タブと文字 */
        snprintf(num, sizeof num, "\t%u\t", ev->tab);
        buf_str(&b, num);
        buf_escaped(&b, ev->text, ev->text_len);
        break;
    }
    buf_str(&b, "\n");
    int ok = fd_write(to_app, b.p, b.len);
    free(b.p);
    return ok;
}

/* ---- 本体の操作 → エンジン。"close" なら 0 ---- */

static int apply(JweEngine *e, char *line) {
    char *f[4] = {line, "", "", ""};
    int n = 1;
    for (char *p = line; *p && n < 4; p++) {
        if (*p == '\t') {
            *p = 0;
            f[n++] = p + 1;
        }
    }
    const char *op = f[0];
    if (strcmp(op, "close") == 0) return 0;
    if (strcmp(op, "chrome_load") == 0) {
        size_t len = unescape(f[2]);
        jwe_chrome_load(e, f[2], (uint32_t)len, (uint32_t)strtoul(f[1], NULL, 10));
    } else if (strcmp(op, "chrome_post") == 0) {
        size_t len = unescape(f[1]);
        jwe_chrome_post(e, f[1], (uint32_t)len);
    } else if (strcmp(op, "chrome_height") == 0) {
        /* "full" は外枠が窓全体を覆う */
        jwe_chrome_height(e, strcmp(f[1], "full") == 0 ? JWE_CHROME_FULL : (uint32_t)strtoul(f[1], NULL, 10));
    } else if (strcmp(op, "set_title") == 0) {
        size_t len = unescape(f[1]);
        jwe_set_title(e, f[1], (uint32_t)len);
    } else if (strcmp(op, "window") == 0) {
        jwe_window(e, (uint32_t)strtoul(f[1], NULL, 10));
    } else if (strcmp(op, "clear_site_data") == 0) {
        jwe_clear_site_data(e);
    } else if (strcmp(op, "copy") == 0) {
        size_t len = unescape(f[1]);
        clip_copy(f[1], len);
        memset(f[1], 0, len);
    } else if (strcmp(op, "fill") == 0) {
        /* ブラウザの窓へ頼む（ブラウザが出どころを確かめる） */
        size_t a = unescape(f[1]), b = unescape(f[2]), c = unescape(f[3]);
        Buf m = {0};
        buf_str(&m, "fill\t");
        buf_escaped(&m, f[1], a);
        buf_str(&m, "\t");
        buf_escaped(&m, f[2], b);
        buf_str(&m, "\t");
        buf_escaped(&m, f[3], c);
        int sent = to_browser(m.p);
        memset(m.p, 0, m.len);
        memset(f[3], 0, c);
        free(m.p);
        if (!sent) send_app("filled\t-1\n");
    } else if (strcmp(op, "ask_site") == 0) {
        to_browser("site?");
    } else {
        fprintf(stderr, "jwvault: 知らない操作: %s\n", op);
    }
    return 1;
}

/* 届いている操作を全部エンジンに渡す。終わるなら 0 */
static int apply_all(JweEngine *e) {
    Line *l;
    while ((l = q_pop()) != NULL) {
        int go = apply(e, l->text);
        free(l);
        if (!go) return 0;
    }
    LOCK();
    int eof = q_eof && !q_head;
    UNLOCK();
    return !eof;
}

/* 偽エンジンのとき: 本体が次のイベントを待ち始めるまで待つ */
static void wait_for_app(void) {
    LOCK();
    while (q_waits == 0 && !q_eof && !q_head) {
#ifdef _WIN32
        UNLOCK();
        WaitForSingleObject(q_signal, INFINITE);
        LOCK();
#else
        pthread_cond_wait(&q_cond, &q_lock);
#endif
    }
    UNLOCK();
}

static int take_wait(void) {
    LOCK();
    int w = q_waits > 0 && !q_head;
    if (w) q_waits--;
    UNLOCK();
    return w;
}

/* データの置き場所: Windows は %LOCALAPPDATA%\jabberwock-browser、ほかは引数か ./browser-data */
static void data_dir(char *out, size_t cap) {
    if (data_arg) {
        snprintf(out, cap, "%s", data_arg);
        return;
    }
#ifdef _WIN32
    wchar_t w[MAX_PATH];
    DWORD n = GetEnvironmentVariableW(L"LOCALAPPDATA", w, MAX_PATH);
    if (n && n < MAX_PATH - 32) {
        wcscat(w, L"\\jabberwock-browser");
        CreateDirectoryW(w, NULL);
        WideCharToMultiByte(CP_UTF8, 0, w, -1, out, (int)cap, NULL, NULL);
        return;
    }
#endif
    snprintf(out, cap, "browser-data");
}

/* 届いたメッセージ（ほかの窓から）を処理する */
static void ipc_apply(JweEngine *e) {
#ifdef _WIN32
    Line *l;
    while ((l = ipc_pop()) != NULL) {
        char *t = l->text;
        size_t n = strlen(t);
        if (strncmp(t, "site\t", 5) == 0) {
            send_app_text("site", t + 5, n - 5);
        } else if (strncmp(t, "filled\t", 7) == 0) {
            char r[48];
            snprintf(r, sizeof r, "filled\t%ld\n", strtol(t + 7, NULL, 10));
            send_app(r);
        } else if (strcmp(t, "show") == 0) {
            jwe_window(e, JWE_WINDOW_FOCUS);
        }
        memset(l->text, 0, n);
        free(l);
    }
#else
    (void)e;
#endif
    clip_tick();
    if (last_use && now_ms() - last_use >= VAULT_IDLE_MS) {
        last_use = 0;
        send_app("lock\n"); /* しばらく触らなければ閉じる */
    }
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--data") == 0 && i + 1 < argc) data_arg = argv[++i];
        /* --vault は前の版の名残り（読み飛ばす） */
    }
    char dir[1024], engine_dir[1100];
    data_dir(dir, sizeof dir);
    /* 金庫の WebView2 の置き場所はブラウザと分ける */
    snprintf(engine_dir, sizeof engine_dir, "%s/vault", dir);
#ifdef _WIN32
    if (ipc_send(IPC_VAULT, "show", 4)) return 0; /* もう開いている */
    for (char *p = engine_dir; *p; p++)
        if (*p == '/') *p = '\\';
    {
        wchar_t w[1100];
        MultiByteToWideChar(CP_UTF8, 0, engine_dir, -1, w, 1100);
        CreateDirectoryW(w, NULL);
    }
    ipc_open();
#endif
    JweConfig cfg;
    memset(&cfg, 0, sizeof cfg);
    cfg.title = "jabberwock 金庫";
    cfg.title_len = (uint32_t)strlen(cfg.title);
    cfg.data_dir = engine_dir;
    cfg.data_dir_len = (uint32_t)strlen(engine_dir);
    cfg.width = 520;
    cfg.height = 640;
    JweEngine *e = NULL;
    JweStatus st = jwe_open(&cfg, &e);
    if (st == JWE_ERR_UNAVAILABLE) {
        fail("WebView2 の実行環境が見つかりません。https://developer.microsoft.com/microsoft-edge/webview2/ から入れてください");
        return 1;
    }
    if (st != JWE_OK) {
        fail("エンジンを開けませんでした");
        return 1;
    }
    JweInfo info;
    jwe_info(e, &info);
    int sync = strcmp(info.name, "mock") == 0;
    if (!start_app(dir)) {
        jwe_close(e);
        return 1;
    }
    {
        /* 最初の行でエンジンの名前とできることを知らせる */
        char first[128];
        snprintf(first, sizeof first, "info\t%s\t%u\n", info.name, info.caps);
        fd_write(to_app, first, strlen(first));
    }
    int running = 1;
    if (sync) wait_for_app(); /* 起動したときの操作を出し切るまで */
    last_use = now_ms();
    while (running) {
        if (!apply_all(e)) break;
        ipc_apply(e);
        if (sync) {
            if (!take_wait()) {
                wait_for_app();
                continue;
            }
        }
        JweEvent ev;
        if (jwe_wait(e, sync ? 0 : 50, &ev)) {
            if (ev.kind == JWE_EV_ERROR && ev.code == JWE_ERR_UNAVAILABLE) {
                fail("WebView2 を起動できませんでした");
                running = 0;
            }
            if (!forward_event(&ev)) running = 0;
        } else if (sync) {
            break; /* 偽エンジンの台本が終わった */
        }
    }
    stop_app();
    jwe_close(e);
    return 0;
}
