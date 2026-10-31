/* 偽エンジンが jw_engine.h の決まりどおりに動くかを確かめる（cc tests/mock_test.c mock/jwe_mock.c） */
#include "../jw_engine.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int fails = 0;
#define CHECK(c) do { if (!(c)) { fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #c); fails++; } } while (0)

static int next(JweEngine *e, JweEvent *ev) { return jwe_poll(e, ev); }

static int text_is(const JweEvent *ev, const char *s) {
    return ev->text_len == strlen(s) && memcmp(ev->text, s, ev->text_len) == 0;
}

/* 次のイベントが kind で、text が s（NULL なら見ない）か */
static void expect(JweEngine *e, uint32_t kind, const char *s, int64_t code) {
    JweEvent ev;
    CHECK(next(e, &ev));
    CHECK(ev.kind == kind);
    if (s) CHECK(text_is(&ev, s));
    if (code >= 0) CHECK(ev.code == code);
}

static void drain_load(JweEngine *e, const char *url, int64_t hist) {
    char title[256];
    snprintf(title, sizeof title, "title:%s", url);
    expect(e, JWE_EV_NAV_STARTED, url, -1);
    expect(e, JWE_EV_URL_CHANGED, url, -1);
    expect(e, JWE_EV_TITLE_CHANGED, title, -1);
    expect(e, JWE_EV_NAV_FINISHED, url, 0);
    expect(e, JWE_EV_HISTORY_CHANGED, NULL, hist);
}

int main(void) {
    JweConfig cfg;
    memset(&cfg, 0, sizeof cfg);
    cfg.title = "t";
    cfg.title_len = 1;
    cfg.ephemeral = 1;
    JweEngine *e = NULL;
    CHECK(jwe_open(&cfg, &e) == JWE_OK);
    JweInfo info;
    jwe_info(e, &info);
    CHECK(info.abi_version == JWE_ABI_VERSION);
    expect(e, JWE_EV_READY, NULL, -1);
    JweEvent ev;
    CHECK(!next(e, &ev));

    uint32_t t;
    CHECK(jwe_tab_new(e, &t) == JWE_OK && t == 1);
    CHECK(jwe_navigate(e, t, "a:1", 3) == JWE_OK);
    drain_load(e, "a:1", 0);
    CHECK(jwe_navigate(e, t, "a:2", 3) == JWE_OK);
    drain_load(e, "a:2", 1);
    CHECK(jwe_back(e, t) == JWE_OK);
    drain_load(e, "a:1", 2);
    CHECK(jwe_navigate(e, t, "a:3", 3) == JWE_OK); /* 進む先は捨てる */
    drain_load(e, "a:3", 1);
    CHECK(jwe_forward(e, t) == JWE_OK);
    CHECK(!next(e, &ev));

    CHECK(jwe_tab_close(e, t) == JWE_OK);
    CHECK(jwe_navigate(e, t, "a:1", 3) == JWE_ERR_BAD_TAB);
    uint32_t t2;
    CHECK(jwe_tab_new(e, &t2) == JWE_OK && t2 == 2); /* 番号は使い回さない */
    CHECK(jwe_chrome_post(e, "x", 1) == JWE_ERR_FAILED);
    CHECK(jwe_chrome_height(e, JWE_CHROME_FULL) == JWE_ERR_FAILED); /* jwe_chrome_load がまだ */

    /* 台本がなければ CLOSE_REQUESTED を 1 回だけ */
    CHECK(jwe_wait(e, -1, &ev) && ev.kind == JWE_EV_CLOSE_REQUESTED);
    CHECK(!jwe_wait(e, 0, &ev));
    jwe_close(e);

    if (fails) { fprintf(stderr, "%d failed\n", fails); return 1; }
    puts("ok");
    return 0;
}
