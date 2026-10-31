/* 手で動かして確かめる小さなブラウザ（jabberwock を使わない）。エンジンが届けるイベントを全部表示する。
 *   smoke [URL]
 * 外枠はアドレス入力欄だけ。Enter で今のタブをその URL へ移す。窓の × で終わる。 */
#include "../../../runtime/engine/jw_engine.h"

#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#endif

/* 起動できなかった理由を、ダブルクリックで開いたときにも見えるように出す */
static void fail(const char *msg) {
    fprintf(stderr, "%s\n", msg);
#ifdef _WIN32
    MessageBoxA(NULL, msg, "smoke", MB_OK | MB_ICONERROR);
#endif
}

static const char CHROME[] =
    "<!doctype html><meta charset=utf-8><body style='margin:0;font:14px sans-serif'>"
    "<input id=a style='width:100%;box-sizing:border-box;height:32px' placeholder='URL'>"
    "<script>a.onkeydown=e=>{if(e.key==='Enter')chrome.webview.postMessage('go '+a.value)};"
    "chrome.webview.addEventListener('message',e=>{a.value=e.data})</script>";

static const char *KINDS[] = {"none", "ready", "close", "nav_started", "nav_finished", "url", "title",
                              "history", "new_tab", "chrome", "key", "permission_denied", "error"};

int main(int argc, char **argv) {
    const char *start = argc > 1 ? argv[1] : "https://example.com/";
    JweConfig cfg;
    memset(&cfg, 0, sizeof cfg);
    cfg.title = "smoke";
    cfg.title_len = 5;
    cfg.data_dir = "smoke-data";
    cfg.data_dir_len = 10;
    cfg.width = 1000;
    cfg.height = 700;
    cfg.ephemeral = 1;
    JweEngine *e;
    JweStatus st = jwe_open(&cfg, &e);
    if (st == JWE_ERR_UNAVAILABLE) {
        fail("WebView2 runtime not found. Install it from https://developer.microsoft.com/microsoft-edge/webview2/ (Evergreen Bootstrapper).");
        return 1;
    }
    if (st != JWE_OK) {
        char msg[64];
        snprintf(msg, sizeof msg, "jwe_open failed: %d", (int)st);
        fail(msg);
        return 1;
    }
    JweInfo info;
    jwe_info(e, &info);
    printf("engine %s %s caps=%u\n", info.name, info.version, info.caps);
    jwe_chrome_load(e, CHROME, sizeof CHROME - 1, 32);
    uint32_t tab;
    jwe_tab_new(e, &tab);
    jwe_tab_show(e, tab);
    jwe_navigate(e, tab, start, (uint32_t)strlen(start));
    JweEvent ev;
    for (;;) {
        if (!jwe_wait(e, 1000, &ev)) continue;
        printf("%-17s tab=%u code=%lld %.*s\n", ev.kind < sizeof KINDS / sizeof *KINDS ? KINDS[ev.kind] : "?",
               ev.tab, (long long)ev.code, (int)ev.text_len, ev.text ? ev.text : "");
        fflush(stdout);
        if (ev.kind == JWE_EV_CLOSE_REQUESTED) break;
        if (ev.kind == JWE_EV_ERROR && ev.code == JWE_ERR_UNAVAILABLE) {
            fail("WebView2 runtime could not start.");
            break;
        }
        if (ev.kind == JWE_EV_CHROME_MESSAGE && ev.text_len > 3 && memcmp(ev.text, "go ", 3) == 0) {
            char url[4096];
            snprintf(url, sizeof url, "%.*s", (int)(ev.text_len - 3), ev.text + 3);
            jwe_navigate(e, tab, url, (uint32_t)strlen(url));
        } else if (ev.kind == JWE_EV_URL_CHANGED && ev.tab == tab) {
            char url[4096];
            snprintf(url, sizeof url, "%.*s", (int)ev.text_len, ev.text);
            jwe_chrome_post(e, url, (uint32_t)strlen(url));
        } else if (ev.kind == JWE_EV_KEY && ev.code == JWE_KEY_BACK) {
            jwe_back(e, tab);
        } else if (ev.kind == JWE_EV_KEY && ev.code == JWE_KEY_FORWARD) {
            jwe_forward(e, tab);
        } else if (ev.kind == JWE_EV_KEY && ev.code == JWE_KEY_RELOAD) {
            jwe_reload(e, tab);
        } else if (ev.kind == JWE_EV_NEW_TAB_REQUESTED) {
            jwe_navigate(e, tab, ev.text, ev.text_len); /* 試作: 同じタブで開く */
        }
    }
    jwe_close(e);
    return 0;
}
