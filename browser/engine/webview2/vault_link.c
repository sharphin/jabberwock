/* vault_link.c: Windows 版のブラウザで、エンジン（jwe_webview2.cpp）と金庫の窓（host/jwvault.c）をつなぐ。
 *
 * ブラウザ本体（jabberwock）は std.browser でエンジンを直接使うが、std にはまだプロセスどうしのやりとりがない。
 * そのあいだのつなぎとして、エンジンの関数のうち 5 つ（open・close・poll・wait・tab_show）をここで包む。
 * 本物は jwe_webview2.cpp を -Djwe_open=jwe_inner_open のように名前を変えてコンパイルしたもの（browser/build-windows.sh）。
 * std にやりとりの機能ができたら、これは消して本体（jabberwock）でする。
 *
 * することは 4 つだけ:
 * - 外枠の 🔑（"vault" の文字列）で、実行ファイルの横の jwvault.exe を起動する（もう開いていれば手前に出す）。
 *   この文字列は本体へは渡さない。
 * - 見えているタブの URL が変わったら、金庫へ「site URL」を送る（金庫が「site?」と聞いたときも）。
 * - 金庫から「fill サイト ユーザー パスワード」が来たら、見えているタブに jwe_fill_login で入れ、結果を「filled 番号」で返す。
 *   出どころの確かめはエンジンがする（サイトが今のページと完全に同じときだけ入れる）。
 * - WebView2 の実行環境がなければ、そう知らせる窓を出す（本体は窓を持たないので）。
 * やりとりは窓のメッセージ（WM_COPYDATA。dwData は 0x6a77）。行の中の \ タブ 改行は \\ \t \n。
 */
#ifndef UNICODE
#define UNICODE
#endif
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

#include "../../../runtime/engine/jw_engine.h"

JweStatus jwe_inner_open(const JweConfig *cfg, JweEngine **out);
void jwe_inner_close(JweEngine *e);
int jwe_inner_poll(JweEngine *e, JweEvent *out);
int jwe_inner_wait(JweEngine *e, int32_t timeout_ms, JweEvent *out);
JweStatus jwe_inner_tab_show(JweEngine *e, uint32_t tab);

static const wchar_t IPC_BROWSER[] = L"JabberwockBrowserIpc";
static const wchar_t IPC_VAULT[] = L"JabberwockVaultIpc";

typedef struct Msg {
    struct Msg *next;
    size_t len;
    char text[];
} Msg;

typedef struct {
    uint32_t tab;
    char *url;
} TabUrl;

static HWND ipc_hwnd;
static Msg *in_head, *in_tail; /* 金庫から届いた行 */
static uint32_t shown;         /* 見えているタブ（エンジンの番号） */
static TabUrl *urls;
static size_t nurls;
static wchar_t data_dir[MAX_PATH]; /* 金庫に渡す置き場所 */

static LRESULT CALLBACK ipc_proc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    if (msg == WM_COPYDATA) {
        const COPYDATASTRUCT *cd = (const COPYDATASTRUCT *)lp;
        if (cd->dwData != 0x6a77 || cd->cbData > 65536) return FALSE;
        Msg *m = (Msg *)malloc(sizeof(Msg) + cd->cbData + 1);
        if (!m) return FALSE;
        memcpy(m->text, cd->lpData, cd->cbData);
        m->text[cd->cbData] = 0;
        m->len = cd->cbData;
        m->next = NULL;
        if (in_tail) in_tail->next = m;
        else in_head = m;
        in_tail = m;
        return TRUE;
    }
    return DefWindowProcW(h, msg, wp, lp);
}

static int send_vault(const char *s, size_t n) {
    HWND h = FindWindowExW(HWND_MESSAGE, NULL, IPC_VAULT, NULL);
    if (!h) return 0;
    COPYDATASTRUCT cd;
    cd.dwData = 0x6a77;
    cd.cbData = (DWORD)n;
    cd.lpData = (void *)s;
    DWORD_PTR r = 0;
    return SendMessageTimeoutW(h, WM_COPYDATA, (WPARAM)ipc_hwnd, (LPARAM)&cd, SMTO_ABORTIFHUNG, 2000, &r) != 0;
}

static const char *url_of(uint32_t tab) {
    for (size_t i = 0; i < nurls; i++)
        if (urls[i].tab == tab) return urls[i].url;
    return "";
}

/* 見えているタブの URL を金庫へ（金庫がなければ何もしない） */
static void tell_site(void) {
    const char *u = url_of(shown);
    size_t n = strlen(u);
    char *b = (char *)malloc(n + 6);
    if (!b) return;
    memcpy(b, "site\t", 5);
    memcpy(b + 5, u, n);
    send_vault(b, n + 5);
    free(b);
}

static void set_url(uint32_t tab, const char *s, uint32_t n) {
    char *u = (char *)malloc((size_t)n + 1);
    if (!u) return;
    memcpy(u, s, n);
    u[n] = 0;
    for (size_t i = 0; i < nurls; i++) {
        if (urls[i].tab == tab) {
            free(urls[i].url);
            urls[i].url = u;
            if (tab == shown) tell_site();
            return;
        }
    }
    TabUrl *nu = (TabUrl *)realloc(urls, sizeof(TabUrl) * (nurls + 1));
    if (!nu) {
        free(u);
        return;
    }
    urls = nu;
    urls[nurls].tab = tab;
    urls[nurls].url = u;
    nurls++;
    if (tab == shown) tell_site();
}

/* その場で \\ \t \n を戻す。長さを返す */
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

/* 金庫の窓を出す（もう開いていれば手前に出す） */
static void open_vault(void) {
    if (send_vault("show", 4)) return;
    wchar_t exe[MAX_PATH];
    GetModuleFileNameW(NULL, exe, MAX_PATH);
    wchar_t *slash = wcsrchr(exe, L'\\');
    if (slash) slash[1] = 0;
    else exe[0] = 0;
    wcsncat(exe, L"jwvault.exe", MAX_PATH - wcslen(exe) - 1);
    if (GetFileAttributesW(exe) == INVALID_FILE_ATTRIBUTES) {
        MessageBoxW(NULL, L"jwvault.exe が見つかりません（jabberwock-browser.exe と同じフォルダに置いてください）", L"jabberwock",
                    MB_OK | MB_ICONERROR);
        return;
    }
    wchar_t cmd[3 * MAX_PATH];
    if (data_dir[0]) _snwprintf(cmd, 3 * MAX_PATH, L"\"%ls\" --data \"%ls\"", exe, data_dir);
    else _snwprintf(cmd, 3 * MAX_PATH, L"\"%ls\"", exe);
    cmd[3 * MAX_PATH - 1] = 0;
    STARTUPINFOW si;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    if (CreateProcessW(exe, cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
}

/* 金庫から届いた行を処理する */
static void handle_ipc(JweEngine *e) {
    Msg *m;
    while ((m = in_head) != NULL) {
        in_head = m->next;
        if (!in_head) in_tail = NULL;
        char *t = m->text;
        if (strncmp(t, "fill\t", 5) == 0) {
            char *f[3] = {t + 5, "", ""};
            int k = 1;
            for (char *p = t + 5; *p && k < 3; p++)
                if (*p == '\t') {
                    *p = 0;
                    f[k++] = p + 1;
                }
            size_t a = unescape(f[0]), b = unescape(f[1]), c = unescape(f[2]);
            JweStatus st = jwe_fill_login(e, shown, f[0], (uint32_t)a, f[1], (uint32_t)b, f[2], (uint32_t)c);
            if (st == JWE_OK) jwe_window(e, JWE_WINDOW_FOCUS);
            char r[32];
            int rn = snprintf(r, sizeof r, "filled\t%d", (int)st);
            send_vault(r, rn > 0 ? (size_t)rn : 0);
        } else if (strcmp(t, "site?") == 0) {
            tell_site();
        }
        SecureZeroMemory(m->text, m->len);
        free(m);
    }
}

/* イベントを見て、金庫に関わるものを処理する。本体へ渡さないなら 0 */
static int watch(const JweEvent *ev) {
    if (ev->kind == JWE_EV_URL_CHANGED || ev->kind == JWE_EV_NAV_FINISHED) set_url(ev->tab, ev->text, ev->text_len);
    if (ev->kind == JWE_EV_CHROME_MESSAGE && ev->text_len == 5 && memcmp(ev->text, "vault", 5) == 0) {
        open_vault();
        return 0;
    }
    return 1;
}

JweStatus jwe_open(const JweConfig *cfg, JweEngine **out) {
    JweStatus st = jwe_inner_open(cfg, out);
    if (st == JWE_ERR_UNAVAILABLE) {
        MessageBoxW(NULL, L"WebView2 の実行環境が見つかりません。https://developer.microsoft.com/microsoft-edge/webview2/ から入れてください",
                    L"jabberwock", MB_OK | MB_ICONERROR);
    }
    if (st != JWE_OK) return st;
    data_dir[0] = 0;
    if (cfg->data_dir_len > 0 && cfg->data_dir_len < 1024) {
        int n = MultiByteToWideChar(CP_UTF8, 0, cfg->data_dir, (int)cfg->data_dir_len, data_dir, MAX_PATH - 1);
        data_dir[n > 0 ? n : 0] = 0;
    }
    WNDCLASSEXW wc;
    memset(&wc, 0, sizeof wc);
    wc.cbSize = sizeof wc;
    wc.lpfnWndProc = ipc_proc;
    wc.hInstance = GetModuleHandleW(NULL);
    wc.lpszClassName = IPC_BROWSER;
    RegisterClassExW(&wc);
    ipc_hwnd = CreateWindowExW(0, IPC_BROWSER, L"", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, wc.hInstance, NULL);
    return st;
}

void jwe_close(JweEngine *e) {
    if (ipc_hwnd) DestroyWindow(ipc_hwnd);
    ipc_hwnd = NULL;
    for (size_t i = 0; i < nurls; i++) free(urls[i].url);
    free(urls);
    urls = NULL;
    nurls = 0;
    jwe_inner_close(e);
}

JweStatus jwe_tab_show(JweEngine *e, uint32_t tab) {
    JweStatus st = jwe_inner_tab_show(e, tab);
    if (st == JWE_OK) {
        shown = tab;
        tell_site();
    }
    return st;
}

int jwe_poll(JweEngine *e, JweEvent *out) {
    for (;;) {
        int got = jwe_inner_poll(e, out);
        handle_ipc(e);
        if (!got) return 0;
        if (watch(out)) return 1;
    }
}

/* 金庫からの行は窓のメッセージで届くので、短く区切って待ち、そのたびに処理する */
int jwe_wait(JweEngine *e, int32_t timeout_ms, JweEvent *out) {
    ULONGLONG end = timeout_ms < 0 ? 0 : GetTickCount64() + (ULONGLONG)timeout_ms;
    for (;;) {
        int32_t slice = 50;
        if (timeout_ms >= 0) {
            ULONGLONG now = GetTickCount64();
            if (now >= end) slice = 0;
            else if (end - now < 50) slice = (int32_t)(end - now);
        }
        int got = jwe_inner_wait(e, slice, out);
        handle_ipc(e);
        if (got) {
            if (watch(out)) return 1;
            continue;
        }
        if (timeout_ms >= 0 && GetTickCount64() >= end) return 0;
    }
}
