// jwe_webview2.cpp: jw_engine.h を WebView2（Windows の Edge のエンジン）で実装する。
//
// 作り:
// - 窓は Win32 の窓 1 つ。上に外枠（chrome）の WebView、下に見えているタブの WebView を置く。
//   タブごとに WebView を 1 つ作り、見えないタブは IsVisible = FALSE にする。
// - WebView2 の呼び返しはすべてイベントを溜めるだけ（jabberwock の関数は呼ばない）。
//   jwe_poll / jwe_wait が OS のメッセージを処理し、溜まったイベントを 1 つずつ渡す。
// - 入っている WebView2 の実行環境の DLL を絶対パスで読む（WebView2Loader.dll は使わない。名前で探すと偽物を拾うため）。
// - アカウント・同期・パスワード保存は使わない。権限の要求（カメラ・位置など）はいつも断る。
//
// ビルド（llvm-mingw の例）:
//   clang++ -std=c++17 -O2 -c jwe_webview2.cpp -I <WebView2 SDK の include>
//   リンク時に -lole32 -luser32 -lshell32 -lshlwapi
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <objbase.h>
#include <shellapi.h>

#include <deque>
#include <functional>
#include <string>
#include <utility>
#include <vector>

#include "WebView2.h"
#include "../../../runtime/engine/jw_engine.h"
#include "../../../runtime/engine/site.h"

namespace {

// ---- 文字の変換 ----

std::wstring widen(const char *s, uint32_t n) {
    if (n == 0) return std::wstring();
    int w = MultiByteToWideChar(CP_UTF8, 0, s, (int)n, nullptr, 0);
    std::wstring r((size_t)w, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s, (int)n, &r[0], w);
    return r;
}

std::string narrow(const wchar_t *s) {
    if (!s || !*s) return std::string();
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string r((size_t)(n - 1), '\0');
    WideCharToMultiByte(CP_UTF8, 0, s, -1, &r[0], n, nullptr, nullptr);
    return r;
}

// WebView2 が CoTaskMemAlloc で渡す文字列を受け取って解放する
std::string take(LPWSTR s) {
    std::string r = narrow(s);
    CoTaskMemFree(s);
    return r;
}

// インターフェースの番号（IID）。SDK のライブラリ（WebView2Guid.lib）や __uuidof に頼らず、
// MSVC でも mingw でも同じようにビルドできるよう、ここに書く。値は WebView2.h の DEFINE_GUID と同じ
constexpr IID kIUnknown = {0x00000000, 0x0000, 0x0000, {0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};
constexpr IID kISettings4 = {0xcb56846c, 0x4168, 0x4d53, {0xb0, 0x4f, 0x03, 0xb6, 0xd6, 0x79, 0x6f, 0xf2}};

// ---- 呼び返しの入れ物（WRL を使わずに書く。引数 2 つの Invoke だけ） ----

#define JWE_HANDLER(Name, I, IID_VALUE, A, B)                                                          \
    class Name final : public I {                                                           \
        LONG refs_ = 0; /* 受け取った側の AddRef で 1 になる */                        \
        std::function<HRESULT(A, B)> f_;                                                    \
        static constexpr IID kIid = IID_VALUE;                                              \
    public:                                                                                 \
        explicit Name(std::function<HRESULT(A, B)> f) : f_(std::move(f)) {}                  \
        HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void **ppv) override {        \
            if (IsEqualIID(riid, kIUnknown) || IsEqualIID(riid, kIid)) {          \
                *ppv = static_cast<I *>(this);                                              \
                AddRef();                                                                   \
                return S_OK;                                                                \
            }                                                                               \
            *ppv = nullptr;                                                                 \
            return E_NOINTERFACE;                                                           \
        }                                                                                   \
        ULONG STDMETHODCALLTYPE AddRef() override { return (ULONG)InterlockedIncrement(&refs_); } \
        ULONG STDMETHODCALLTYPE Release() override {                                        \
            LONG r = InterlockedDecrement(&refs_);                                          \
            if (r == 0) delete this;                                                        \
            return (ULONG)r;                                                                \
        }                                                                                   \
        HRESULT STDMETHODCALLTYPE Invoke(A a, B b) override { return f_(a, b); }            \
    };

JWE_HANDLER(EnvDone, ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler, (IID{0x4e8a3389, 0xc9d8, 0x4bd2, {0xb6, 0xb5, 0x12, 0x4f, 0xee, 0x6c, 0xc1, 0x4d}}), HRESULT, ICoreWebView2Environment *)
JWE_HANDLER(CtlDone, ICoreWebView2CreateCoreWebView2ControllerCompletedHandler, (IID{0x6c4819f3, 0xc9b7, 0x4260, {0x81, 0x27, 0xc9, 0xf5, 0xbd, 0xe7, 0xf6, 0x8c}}), HRESULT, ICoreWebView2Controller *)
JWE_HANDLER(NavStarting, ICoreWebView2NavigationStartingEventHandler, (IID{0x9adbe429, 0xf36d, 0x432b, {0x9d, 0xdc, 0xf8, 0x88, 0x1f, 0xbd, 0x76, 0xe3}}), ICoreWebView2 *, ICoreWebView2NavigationStartingEventArgs *)
JWE_HANDLER(NavCompleted, ICoreWebView2NavigationCompletedEventHandler, (IID{0xd33a35bf, 0x1c49, 0x4f98, {0x93, 0xab, 0x00, 0x6e, 0x05, 0x33, 0xfe, 0x1c}}), ICoreWebView2 *, ICoreWebView2NavigationCompletedEventArgs *)
JWE_HANDLER(SourceChanged, ICoreWebView2SourceChangedEventHandler, (IID{0x3c067f9f, 0x5388, 0x4772, {0x8b, 0x48, 0x79, 0xf7, 0xef, 0x1a, 0xb3, 0x7c}}), ICoreWebView2 *, ICoreWebView2SourceChangedEventArgs *)
JWE_HANDLER(TitleChanged, ICoreWebView2DocumentTitleChangedEventHandler, (IID{0xf5f2b923, 0x953e, 0x4042, {0x9f, 0x95, 0xf3, 0xa1, 0x18, 0xe1, 0xaf, 0xd4}}), ICoreWebView2 *, IUnknown *)
JWE_HANDLER(HistoryChanged, ICoreWebView2HistoryChangedEventHandler, (IID{0xc79a420c, 0xefd9, 0x4058, {0x92, 0x95, 0x3e, 0x8b, 0x4b, 0xca, 0xb6, 0x45}}), ICoreWebView2 *, IUnknown *)
JWE_HANDLER(NewWindow, ICoreWebView2NewWindowRequestedEventHandler, (IID{0xd4c185fe, 0xc81c, 0x4989, {0x97, 0xaf, 0x2d, 0x3f, 0xa7, 0xab, 0x56, 0x51}}), ICoreWebView2 *, ICoreWebView2NewWindowRequestedEventArgs *)
JWE_HANDLER(Permission, ICoreWebView2PermissionRequestedEventHandler, (IID{0x15e1c6a3, 0xc72a, 0x4df3, {0x91, 0xd7, 0xd0, 0x97, 0xfb, 0xec, 0x6b, 0xfd}}), ICoreWebView2 *, ICoreWebView2PermissionRequestedEventArgs *)
JWE_HANDLER(WebMessage, ICoreWebView2WebMessageReceivedEventHandler, (IID{0x57213f19, 0x00e6, 0x49fa, {0x8e, 0x07, 0x89, 0x8e, 0xa0, 0x1e, 0xcb, 0xd2}}), ICoreWebView2 *, ICoreWebView2WebMessageReceivedEventArgs *)
JWE_HANDLER(AccelKey, ICoreWebView2AcceleratorKeyPressedEventHandler, (IID{0xb29c7e28, 0xfa79, 0x41a8, {0x8e, 0x44, 0x65, 0x81, 0x1c, 0x76, 0xdc, 0xb2}}), ICoreWebView2Controller *, ICoreWebView2AcceleratorKeyPressedEventArgs *)
JWE_HANDLER(ScriptDone, ICoreWebView2ExecuteScriptCompletedHandler, (IID{0x49511172, 0xcc67, 0x4bca, {0x99, 0x23, 0x13, 0x71, 0x12, 0xf4, 0xc4, 0xcc}}), HRESULT, LPCWSTR)
JWE_HANDLER(DevToolsDone, ICoreWebView2CallDevToolsProtocolMethodCompletedHandler, (IID{0x5c4889f0, 0x5ef6, 0x4c5a, {0x95, 0x2c, 0xd8, 0xf1, 0xb9, 0x2d, 0x05, 0x74}}), HRESULT, LPCWSTR)

// 入っている実行環境の DLL を直接呼ぶための入口
// （WebView2Loader.dll が中でしていることと同じ。webview/webview の組み込みの loader と同じ方法）
typedef HRESULT(STDMETHODCALLTYPE *CreateInternalFn)(bool, int, PCWSTR, IUnknown *,
                                                     ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler *);

const wchar_t kClass[] = L"JabberwockBrowser";

struct Tab {
    uint32_t id = 0;
    bool closed = false;
    ICoreWebView2Controller *ctl = nullptr;
    ICoreWebView2 *wv = nullptr;
    std::wstring pending; // 準備ができる前に頼まれた URL
};

struct Queued {
    uint32_t kind;
    uint32_t tab;
    int64_t code;
    std::string text;
};

} // namespace

struct JweEngine {
    HWND hwnd = nullptr;
    HMODULE loader = nullptr;
    ICoreWebView2Environment *env = nullptr;
    std::deque<Queued> queue;
    Queued last; // 最後に渡したイベント（文字列はここが持つ）
    std::vector<Tab> tabs;
    uint32_t shown = 0;
    uint32_t next_id = 1;
    // 外枠
    ICoreWebView2Controller *chrome_ctl = nullptr;
    ICoreWebView2 *chrome = nullptr;
    std::wstring chrome_html;
    uint32_t chrome_height = 0;
    bool chrome_wanted = false;
    bool chrome_ready = false;               // 外枠のページを読み終えた
    std::vector<std::wstring> chrome_pending; // それまでに頼まれた chrome_post
    std::wstring data_dir;
    bool ephemeral = false;
    bool com = false;

    void push(uint32_t kind, uint32_t tab, int64_t code, std::string text = std::string()) {
        queue.push_back(Queued{kind, tab, code, std::move(text)});
    }

    Tab *find(uint32_t id) {
        for (auto &t : tabs)
            if (t.id == id && !t.closed) return &t;
        return nullptr;
    }

    double scale() const {
        UINT dpi = GetDpiForWindow(hwnd);
        return dpi ? dpi / 96.0 : 1.0;
    }

    void layout() {
        RECT rc;
        GetClientRect(hwnd, &rc);
        bool full = chrome_ctl && chrome_height == JWE_CHROME_FULL; // 外枠が窓全体を覆う
        LONG top = !chrome_ctl ? 0 : full ? rc.bottom - rc.top : (LONG)(chrome_height * scale());
        if (chrome_ctl) {
            RECT c = rc;
            c.bottom = rc.top + top;
            chrome_ctl->put_Bounds(c);
        }
        RECT b = rc;
        b.top += top;
        for (auto &t : tabs) {
            if (t.closed || !t.ctl) continue;
            t.ctl->put_Bounds(b);
            t.ctl->put_IsVisible(t.id == shown && !full ? TRUE : FALSE);
        }
    }

    // 窓のショートカットを呼ぶ側に渡す（WebView の中で押されたものも）
    void hook_keys(ICoreWebView2Controller *ctl) {
        EventRegistrationToken tok;
        ctl->add_AcceleratorKeyPressed(new AccelKey([this](ICoreWebView2Controller *, ICoreWebView2AcceleratorKeyPressedEventArgs *a) {
            COREWEBVIEW2_KEY_EVENT_KIND kind;
            UINT key = 0;
            a->get_KeyEventKind(&kind);
            a->get_VirtualKey(&key);
            if (kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN) return S_OK;
            bool ctrl = GetKeyState(VK_CONTROL) < 0, shift = GetKeyState(VK_SHIFT) < 0, alt = GetKeyState(VK_MENU) < 0;
            int code = 0;
            if (ctrl && key == 'L') code = JWE_KEY_FOCUS_ADDRESS;
            else if (ctrl && key == 'T') code = JWE_KEY_NEW_TAB;
            else if (ctrl && key == 'W') code = JWE_KEY_CLOSE_TAB;
            else if (ctrl && key == VK_TAB) code = shift ? JWE_KEY_PREV_TAB : JWE_KEY_NEXT_TAB;
            else if (key == VK_F5 || (ctrl && key == 'R')) code = JWE_KEY_RELOAD;
            else if (alt && key == VK_LEFT) code = JWE_KEY_BACK;
            else if (alt && key == VK_RIGHT) code = JWE_KEY_FORWARD;
            if (code) {
                a->put_Handled(TRUE);
                push(JWE_EV_KEY, 0, code);
            }
            return S_OK;
        }), &tok);
    }

    void settings(ICoreWebView2 *wv, bool chrome_view) {
        ICoreWebView2Settings *s = nullptr;
        if (FAILED(wv->get_Settings(&s)) || !s) return;
        s->put_IsStatusBarEnabled(chrome_view ? FALSE : TRUE);
        s->put_IsZoomControlEnabled(chrome_view ? FALSE : TRUE);
        s->put_AreDefaultContextMenusEnabled(chrome_view ? FALSE : TRUE);
        s->put_IsWebMessageEnabled(chrome_view ? TRUE : FALSE);
        s->put_AreHostObjectsAllowed(FALSE);
#ifdef __ICoreWebView2Settings4_INTERFACE_DEFINED__
        // フォームの自動入力とパスワードの保存は使わない（SDK に定義があるときだけ）
        ICoreWebView2Settings4 *s4 = nullptr;
        if (SUCCEEDED(s->QueryInterface(kISettings4, (void **)&s4)) && s4) {
            s4->put_IsPasswordAutosaveEnabled(FALSE);
            s4->put_IsGeneralAutofillEnabled(FALSE);
            s4->Release();
        }
#endif
        s->Release();
    }

    void hook_tab(Tab &t) {
        uint32_t id = t.id;
        ICoreWebView2 *wv = t.wv;
        EventRegistrationToken tok;
        settings(wv, false);
        wv->add_NavigationStarting(new NavStarting([this, id](ICoreWebView2 *, ICoreWebView2NavigationStartingEventArgs *a) {
            LPWSTR uri = nullptr;
            a->get_Uri(&uri);
            push(JWE_EV_NAV_STARTED, id, 0, take(uri));
            return S_OK;
        }), &tok);
        wv->add_NavigationCompleted(new NavCompleted([this, id](ICoreWebView2 *w, ICoreWebView2NavigationCompletedEventArgs *a) {
            BOOL ok = FALSE;
            COREWEBVIEW2_WEB_ERROR_STATUS st = COREWEBVIEW2_WEB_ERROR_STATUS_UNKNOWN;
            a->get_IsSuccess(&ok);
            if (!ok) a->get_WebErrorStatus(&st);
            LPWSTR src = nullptr;
            w->get_Source(&src);
            // 失敗のときの code は WebErrorStatus（0 にならないよう +1 する）
            push(JWE_EV_NAV_FINISHED, id, ok ? 0 : (int64_t)st + 1, take(src));
            return S_OK;
        }), &tok);
        wv->add_SourceChanged(new SourceChanged([this, id](ICoreWebView2 *w, ICoreWebView2SourceChangedEventArgs *) {
            LPWSTR src = nullptr;
            w->get_Source(&src);
            push(JWE_EV_URL_CHANGED, id, 0, take(src));
            return S_OK;
        }), &tok);
        wv->add_DocumentTitleChanged(new TitleChanged([this, id](ICoreWebView2 *w, IUnknown *) {
            LPWSTR title = nullptr;
            w->get_DocumentTitle(&title);
            push(JWE_EV_TITLE_CHANGED, id, 0, take(title));
            return S_OK;
        }), &tok);
        wv->add_HistoryChanged(new HistoryChanged([this, id](ICoreWebView2 *w, IUnknown *) {
            BOOL b = FALSE, f = FALSE;
            w->get_CanGoBack(&b);
            w->get_CanGoForward(&f);
            push(JWE_EV_HISTORY_CHANGED, id, (b ? 1 : 0) | (f ? 2 : 0));
            return S_OK;
        }), &tok);
        // 新しい窓は開かず、呼ぶ側にタブとして開くかを任せる
        wv->add_NewWindowRequested(new NewWindow([this, id](ICoreWebView2 *, ICoreWebView2NewWindowRequestedEventArgs *a) {
            LPWSTR uri = nullptr;
            a->get_Uri(&uri);
            a->put_Handled(TRUE);
            push(JWE_EV_NEW_TAB_REQUESTED, id, 0, take(uri));
            return S_OK;
        }), &tok);
        wv->add_PermissionRequested(new Permission([this, id](ICoreWebView2 *, ICoreWebView2PermissionRequestedEventArgs *a) {
            COREWEBVIEW2_PERMISSION_KIND kind = COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION;
            a->get_PermissionKind(&kind);
            a->put_State(COREWEBVIEW2_PERMISSION_STATE_DENY);
            static const char *names[] = {"unknown", "microphone", "camera", "geolocation", "notifications", "other-sensors", "clipboard-read"};
            const char *n = (unsigned)kind < sizeof names / sizeof *names ? names[kind] : "unknown";
            push(JWE_EV_PERMISSION_DENIED, id, (int64_t)kind, n);
            return S_OK;
        }), &tok);
    }

    void create_tab_view(uint32_t id) {
        env->CreateCoreWebView2Controller(hwnd, new CtlDone([this, id](HRESULT hr, ICoreWebView2Controller *ctl) {
            Tab *t = find(id);
            if (FAILED(hr) || !ctl) {
                push(JWE_EV_ERROR, id, JWE_ERR_FAILED, "タブを作れませんでした");
                return S_OK;
            }
            if (!t) { // 準備ができる前に閉じられた
                ctl->Close();
                return S_OK;
            }
            ctl->AddRef();
            t->ctl = ctl;
            ctl->get_CoreWebView2(&t->wv);
            hook_keys(ctl);
            hook_tab(*t);
            layout();
            if (!t->pending.empty()) {
                t->wv->Navigate(t->pending.c_str());
                t->pending.clear();
            }
            return S_OK;
        }));
    }

    void create_chrome_view() {
        env->CreateCoreWebView2Controller(hwnd, new CtlDone([this](HRESULT hr, ICoreWebView2Controller *ctl) {
            if (FAILED(hr) || !ctl) {
                push(JWE_EV_ERROR, 0, JWE_ERR_FAILED, "外枠を作れませんでした");
                return S_OK;
            }
            ctl->AddRef();
            chrome_ctl = ctl;
            ctl->get_CoreWebView2(&chrome);
            settings(chrome, true);
            hook_keys(ctl);
            EventRegistrationToken tok;
            chrome->add_WebMessageReceived(new WebMessage([this](ICoreWebView2 *, ICoreWebView2WebMessageReceivedEventArgs *a) {
                LPWSTR s = nullptr;
                if (SUCCEEDED(a->TryGetWebMessageAsString(&s))) push(JWE_EV_CHROME_MESSAGE, 0, 0, take(s));
                return S_OK;
            }), &tok);
            // 外枠からほかのページへは移らない
            chrome->add_NavigationStarting(new NavStarting([](ICoreWebView2 *, ICoreWebView2NavigationStartingEventArgs *a) {
                LPWSTR uri = nullptr;
                a->get_Uri(&uri);
                bool inline_page = uri && wcsncmp(uri, L"data:", 5) == 0;
                if (uri) CoTaskMemFree(uri);
                if (!inline_page) a->put_Cancel(TRUE);
                return S_OK;
            }), &tok);
            // 外枠のページを読み終えたら、それまでに溜めたものを送る
            chrome->add_NavigationCompleted(new NavCompleted([this](ICoreWebView2 *, ICoreWebView2NavigationCompletedEventArgs *) {
                chrome_ready = true;
                for (const auto &m : chrome_pending) chrome->PostWebMessageAsString(m.c_str());
                chrome_pending.clear();
                return S_OK;
            }), &tok);
            chrome->NavigateToString(chrome_html.c_str());
            layout();
            return S_OK;
        }));
    }

    void on_env(ICoreWebView2Environment *e) {
        e->AddRef();
        env = e;
        if (chrome_wanted) create_chrome_view();
        for (auto &t : tabs)
            if (!t.closed) create_tab_view(t.id);
        push(JWE_EV_READY, 0, 0);
    }
};

namespace {

// "130.0.2849.80" のような版を比べる（a が新しければ正）
int compare_version(const std::wstring &a, const std::wstring &b) {
    size_t i = 0, j = 0;
    while (i < a.size() || j < b.size()) {
        unsigned long x = 0, y = 0;
        while (i < a.size() && a[i] != L'.') x = x * 10 + (unsigned long)(a[i++] - L'0');
        while (j < b.size() && b[j] != L'.') y = y * 10 + (unsigned long)(b[j++] - L'0');
        if (x != y) return x > y ? 1 : -1;
        i++;
        j++;
    }
    return 0;
}

std::wstring env_var(const wchar_t *name) {
    wchar_t buf[MAX_PATH];
    DWORD n = GetEnvironmentVariableW(name, buf, MAX_PATH);
    return n && n < MAX_PATH ? std::wstring(buf, n) : std::wstring();
}

// 入っている WebView2 の実行環境のうち、いちばん新しい版の EmbeddedBrowserWebView.dll を探す
std::wstring find_runtime_dll() {
#if defined(_M_ARM64) || defined(__aarch64__)
    const wchar_t *arch = L"arm64";
#else
    const wchar_t *arch = L"x64";
#endif
    std::wstring bases[] = {env_var(L"ProgramFiles(x86)"), env_var(L"ProgramFiles"), env_var(L"LOCALAPPDATA")};
    std::wstring best, best_ver;
    for (const auto &b : bases) {
        if (b.empty()) continue;
        std::wstring app = b + L"\\Microsoft\\EdgeWebView\\Application";
        WIN32_FIND_DATAW fd;
        HANDLE h = FindFirstFileW((app + L"\\*").c_str(), &fd);
        if (h == INVALID_HANDLE_VALUE) continue;
        do {
            if (!(fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) || fd.cFileName[0] < L'0' || fd.cFileName[0] > L'9') continue;
            std::wstring dll = app + L"\\" + fd.cFileName + L"\\EBWebView\\" + arch + L"\\EmbeddedBrowserWebView.dll";
            if (GetFileAttributesW(dll.c_str()) == INVALID_FILE_ATTRIBUTES) continue;
            if (best.empty() || compare_version(fd.cFileName, best_ver) > 0) {
                best = dll;
                best_ver = fd.cFileName;
            }
        } while (FindNextFileW(h, &fd));
        FindClose(h);
    }
    return best;
}

std::wstring full_path(const std::wstring &p) {
    wchar_t buf[MAX_PATH];
    DWORD n = GetFullPathNameW(p.c_str(), MAX_PATH, buf, nullptr);
    return n && n < MAX_PATH ? std::wstring(buf, n) : p;
}

LRESULT CALLBACK wndproc(HWND h, UINT msg, WPARAM wp, LPARAM lp) {
    JweEngine *e = (JweEngine *)GetWindowLongPtrW(h, GWLP_USERDATA);
    switch (msg) {
    case WM_CLOSE:
        if (e) e->push(JWE_EV_CLOSE_REQUESTED, 0, 0);
        return 0; // 閉じるかどうかは呼ぶ側が決める（jwe_close）
    case WM_NCCALCSIZE:
        // OS のタイトルバーをなくし、外枠（タブ列）を窓の一番上まで広げる。左右と下の縁（大きさを変えるところ）は残す
        if (wp) {
            NCCALCSIZE_PARAMS *p = (NCCALCSIZE_PARAMS *)lp;
            LONG top = p->rgrc[0].top;
            LRESULT r = DefWindowProcW(h, msg, wp, lp);
            p->rgrc[0].top = top;
            if (IsZoomed(h)) {
                // 最大化すると窓は画面の外へ縁の分だけはみ出すので、その分下げる
                UINT dpi = GetDpiForWindow(h);
                p->rgrc[0].top += GetSystemMetricsForDpi(SM_CYFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
            }
            return r;
        }
        break;
    case WM_SIZE:
        if (e) e->layout();
        return 0;
    case WM_DPICHANGED: {
        RECT *r = (RECT *)lp;
        SetWindowPos(h, nullptr, r->left, r->top, r->right - r->left, r->bottom - r->top, SWP_NOZORDER | SWP_NOACTIVATE);
        return 0;
    }
    }
    return DefWindowProcW(h, msg, wp, lp);
}

// 中身ごとフォルダを消す（失敗しても続ける）
void remove_tree(const std::wstring &dir) {
    if (dir.empty()) return;
    std::wstring from = dir;
    from.push_back(L'\0'); // SHFileOperation は 0 が 2 つで終わる
    SHFILEOPSTRUCTW op = {};
    op.wFunc = FO_DELETE;
    op.pFrom = from.c_str();
    op.fFlags = FOF_NO_UI;
    SHFileOperationW(&op);
}

// 前回の一時フォルダ（ephemeral の残り）を消す
void sweep_ephemeral(const std::wstring &base) {
    WIN32_FIND_DATAW fd;
    HANDLE h = FindFirstFileW((base + L"\\ephemeral-*").c_str(), &fd);
    if (h == INVALID_HANDLE_VALUE) return;
    do {
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) remove_tree(base + L"\\" + fd.cFileName);
    } while (FindNextFileW(h, &fd));
    FindClose(h);
}

bool pump(JweEngine *e) {
    MSG m;
    bool any = false;
    while (PeekMessageW(&m, nullptr, 0, 0, PM_REMOVE)) {
        TranslateMessage(&m);
        DispatchMessageW(&m);
        any = true;
    }
    (void)e;
    return any;
}

int pop(JweEngine *e, JweEvent *out) {
    if (e->queue.empty()) return 0;
    e->last = std::move(e->queue.front());
    e->queue.pop_front();
    out->kind = e->last.kind;
    out->tab = e->last.tab;
    out->code = e->last.code;
    out->text = e->last.text.data();
    out->text_len = (uint32_t)e->last.text.size();
    return 1;
}

} // namespace

extern "C" {

JweStatus jwe_open(const JweConfig *cfg, JweEngine **out) {
    if (!cfg || !out) return JWE_ERR_BAD_ARG;
    *out = nullptr;
    JweEngine *e = new JweEngine();
    HRESULT hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    e->com = SUCCEEDED(hr);
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    // Windows に入っている WebView2 の実行環境（Program Files の中）の DLL を、絶対パスで直接読む。
    // WebView2Loader.dll は名前で探すと、実行ファイルの横（ダウンロードのフォルダなど）に置かれた偽物を拾うので使わない
    CreateInternalFn create_internal = nullptr;
    std::wstring dll = find_runtime_dll();
    if (!dll.empty()) {
        // その DLL が頼る DLL も、DLL のフォルダと System32 からだけ探す
        e->loader = LoadLibraryExW(dll.c_str(), nullptr, 0x100 /* LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR */ | 0x800 /* LOAD_LIBRARY_SEARCH_SYSTEM32 */);
        if (!e->loader) e->loader = LoadLibraryW(dll.c_str());
    }
    if (e->loader) create_internal = (CreateInternalFn)(void *)GetProcAddress(e->loader, "CreateWebViewEnvironmentWithOptionsInternal");
    if (!create_internal) {
        jwe_close(e);
        // 呼ぶ側は窓を持たない（コンソールもない）ことが多いので、ここで知らせる
        MessageBoxW(nullptr, L"WebView2 の実行環境が見つかりません。https://developer.microsoft.com/microsoft-edge/webview2/ から入れてください",
                    L"jabberwock", MB_OK | MB_ICONERROR);
        return JWE_ERR_UNAVAILABLE;
    }

    HINSTANCE inst = GetModuleHandleW(nullptr);
    WNDCLASSEXW wc = {};
    wc.cbSize = sizeof wc;
    wc.lpfnWndProc = wndproc;
    wc.hInstance = inst;
    wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    wc.lpszClassName = kClass;
    RegisterClassExW(&wc); // 2 回目は失敗するが、登録済みなので構わない
    std::wstring title = widen(cfg->title, cfg->title_len);
    e->hwnd = CreateWindowExW(0, kClass, title.c_str(), WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT,
                              CW_USEDEFAULT, CW_USEDEFAULT, nullptr, nullptr, inst, nullptr);
    if (!e->hwnd) {
        jwe_close(e);
        return JWE_ERR_FAILED;
    }
    SetWindowLongPtrW(e->hwnd, GWLP_USERDATA, (LONG_PTR)e);
    if (cfg->width && cfg->height) {
        double s = e->scale();
        SetWindowPos(e->hwnd, nullptr, 0, 0, (int)(cfg->width * s), (int)(cfg->height * s), SWP_NOMOVE | SWP_NOZORDER);
    }
    SetWindowPos(e->hwnd, nullptr, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER); // WM_NCCALCSIZE を効かせる
    ShowWindow(e->hwnd, SW_SHOW);

    std::wstring base = widen(cfg->data_dir, cfg->data_dir_len);
    e->ephemeral = cfg->ephemeral != 0;
    if (base.empty()) base = env_var(L"TEMP") + L"\\jabberwock-browser"; // 置き場所はいつも 1 か所に決める
    base = full_path(base);
    {
        CreateDirectoryW(base.c_str(), nullptr);
        sweep_ephemeral(base);
        e->data_dir = e->ephemeral ? base + L"\\ephemeral-" + std::to_wstring(GetCurrentProcessId()) : base + L"\\webview2";
    }
    EnvDone *done = new EnvDone([e](HRESULT r, ICoreWebView2Environment *env) {
        if (FAILED(r) || !env) {
            e->push(JWE_EV_ERROR, 0, JWE_ERR_UNAVAILABLE, "WebView2 の実行環境を使えませんでした");
            return S_OK;
        }
        e->on_env(env);
        return S_OK;
    });
    hr = create_internal(true, 0 /* 入っている実行環境 */, e->data_dir.c_str(), nullptr, done);
    if (FAILED(hr)) {
        jwe_close(e);
        return JWE_ERR_UNAVAILABLE;
    }
    *out = e;
    return JWE_OK;
}

void jwe_close(JweEngine *e) {
    if (!e) return;
    for (auto &t : e->tabs) {
        if (t.wv) t.wv->Release();
        if (t.ctl) {
            t.ctl->Close();
            t.ctl->Release();
        }
    }
    if (e->chrome) e->chrome->Release();
    if (e->chrome_ctl) {
        e->chrome_ctl->Close();
        e->chrome_ctl->Release();
    }
    if (e->env) e->env->Release();
    if (e->hwnd) {
        SetWindowLongPtrW(e->hwnd, GWLP_USERDATA, 0);
        DestroyWindow(e->hwnd);
    }
    // ブラウザのプロセスが終わるまでフォルダを消せないことがある。残りは次に開いたとき消す
    if (e->ephemeral) remove_tree(e->data_dir);
    if (e->loader) FreeLibrary(e->loader);
    if (e->com) CoUninitialize();
    delete e;
}

void jwe_info(const JweEngine *e, JweInfo *out) {
    out->abi_version = JWE_ABI_VERSION;
    out->caps = JWE_CAP_SCRIPT | JWE_CAP_CHROME_VIEW | JWE_CAP_FRAMELESS | (e && !e->ephemeral ? JWE_CAP_PERSIST : 0);
    out->name = "webview2";
    out->version = "1";
}

int jwe_poll(JweEngine *e, JweEvent *out) {
    pump(e);
    return pop(e, out);
}

int jwe_wait(JweEngine *e, int32_t timeout_ms, JweEvent *out) {
    ULONGLONG end = timeout_ms < 0 ? 0 : GetTickCount64() + (ULONGLONG)timeout_ms;
    for (;;) {
        pump(e);
        if (pop(e, out)) return 1;
        DWORD wait = INFINITE;
        if (timeout_ms >= 0) {
            ULONGLONG now = GetTickCount64();
            if (now >= end) return 0;
            wait = (DWORD)(end - now);
        }
        MsgWaitForMultipleObjectsEx(0, nullptr, wait, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
    }
}

JweStatus jwe_tab_new(JweEngine *e, uint32_t *out_tab) {
    Tab t;
    t.id = e->next_id++;
    e->tabs.push_back(t);
    if (e->env) e->create_tab_view(t.id);
    *out_tab = t.id;
    return JWE_OK;
}

JweStatus jwe_tab_close(JweEngine *e, uint32_t tab) {
    Tab *t = e->find(tab);
    if (!t) return JWE_ERR_BAD_TAB;
    t->closed = true;
    if (t->wv) {
        t->wv->Release();
        t->wv = nullptr;
    }
    if (t->ctl) {
        t->ctl->Close();
        t->ctl->Release();
        t->ctl = nullptr;
    }
    if (e->shown == tab) e->shown = 0;
    return JWE_OK;
}

JweStatus jwe_tab_show(JweEngine *e, uint32_t tab) {
    if (!e->find(tab)) return JWE_ERR_BAD_TAB;
    e->shown = tab;
    e->layout();
    return JWE_OK;
}

JweStatus jwe_navigate(JweEngine *e, uint32_t tab, const char *url, uint32_t url_len) {
    Tab *t = e->find(tab);
    if (!t) return JWE_ERR_BAD_TAB;
    if (!url || url_len == 0) return JWE_ERR_BAD_ARG;
    std::wstring w = widen(url, url_len);
    if (!t->wv) {
        t->pending = w;
        return JWE_OK;
    }
    return SUCCEEDED(t->wv->Navigate(w.c_str())) ? JWE_OK : JWE_ERR_BAD_ARG;
}

#define JWE_TAB_CALL(name, call)                         \
    JweStatus name(JweEngine *e, uint32_t tab) {         \
        Tab *t = e->find(tab);                           \
        if (!t) return JWE_ERR_BAD_TAB;                  \
        if (t->wv) t->wv->call();                        \
        return JWE_OK;                                   \
    }
JWE_TAB_CALL(jwe_back, GoBack)
JWE_TAB_CALL(jwe_forward, GoForward)
JWE_TAB_CALL(jwe_reload, Reload)
JWE_TAB_CALL(jwe_stop, Stop)

JweStatus jwe_chrome_load(JweEngine *e, const char *html, uint32_t html_len, uint32_t height) {
    e->chrome_html = widen(html, html_len);
    e->chrome_height = height;
    if (e->chrome) {
        e->chrome_ready = false;
        e->chrome->NavigateToString(e->chrome_html.c_str());
        e->layout();
    } else if (!e->chrome_wanted) {
        e->chrome_wanted = true;
        if (e->env) e->create_chrome_view();
    }
    return JWE_OK;
}

JweStatus jwe_chrome_post(JweEngine *e, const char *msg, uint32_t msg_len) {
    if (!e->chrome_wanted) return JWE_ERR_FAILED; // jwe_chrome_load がまだ
    std::wstring w = widen(msg, msg_len);
    if (!e->chrome || !e->chrome_ready) {
        // 準備ができるまで溜める（多すぎたら古いものを捨てる。状態はいつも全部を送るので、新しいものだけで足りる）
        if (e->chrome_pending.size() >= 16) e->chrome_pending.erase(e->chrome_pending.begin());
        e->chrome_pending.push_back(w);
        return JWE_OK;
    }
    // アドレス欄へ移すときは、キーボードの入力先も外枠へ移す
    if (w == L"focus" && e->chrome_ctl) e->chrome_ctl->MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
    return SUCCEEDED(e->chrome->PostWebMessageAsString(w.c_str())) ? JWE_OK : JWE_ERR_FAILED;
}

JweStatus jwe_chrome_height(JweEngine *e, uint32_t height) {
    if (!e->chrome_wanted) return JWE_ERR_FAILED; // jwe_chrome_load がまだ
    e->chrome_height = height;
    e->layout();
    return JWE_OK;
}

JweStatus jwe_window(JweEngine *e, uint32_t op) {
    switch (op) {
    case JWE_WINDOW_DRAG:
        // 外枠の中で押されたボタンを、タイトルバーを押したことにして OS に窓を動かさせる
        // （wry / Tauri の窓を動かす方法と同じ）
        {
            POINT pt;
            GetCursorPos(&pt);
            ReleaseCapture();
            PostMessageW(e->hwnd, WM_NCLBUTTONDOWN, HTCAPTION, MAKELPARAM(pt.x, pt.y));
        }
        return JWE_OK;
    case JWE_WINDOW_MINIMIZE:
        ShowWindow(e->hwnd, SW_MINIMIZE);
        return JWE_OK;
    case JWE_WINDOW_TOGGLE_MAXIMIZE:
        ShowWindow(e->hwnd, IsZoomed(e->hwnd) ? SW_RESTORE : SW_MAXIMIZE);
        return JWE_OK;
    case JWE_WINDOW_FOCUS:
        if (!IsWindowVisible(e->hwnd)) ShowWindow(e->hwnd, SW_SHOW);
        if (IsIconic(e->hwnd)) ShowWindow(e->hwnd, SW_RESTORE);
        SetForegroundWindow(e->hwnd);
        return JWE_OK;
    case JWE_WINDOW_HIDE:
        ShowWindow(e->hwnd, SW_HIDE);
        return JWE_OK;
    }
    return JWE_ERR_BAD_ARG;
}

JweStatus jwe_set_title(JweEngine *e, const char *title, uint32_t title_len) {
    std::wstring w = widen(title, title_len);
    return SetWindowTextW(e->hwnd, w.c_str()) ? JWE_OK : JWE_ERR_FAILED;
}

namespace {
// JavaScript の文字列の書き方（"..."）にする
std::wstring js_string(const std::wstring &s) {
    std::wstring r = L"\"";
    for (wchar_t c : s) {
        if (c == L'\\' || c == L'"') { r += L'\\'; r += c; }
        else if (c < 0x20 || c == 0x2028 || c == 0x2029 || c == L'<') {
            wchar_t buf[8];
            swprintf(buf, 8, L"\\u%04x", (unsigned)c);
            r += buf;
        } else r += c;
    }
    return r + L"\"";
}
}

JweStatus jwe_fill_login(JweEngine *e, uint32_t tab, const char *site, uint32_t site_len,
                         const char *user, uint32_t user_len, const char *password, uint32_t password_len) {
    Tab *t = e->find(tab);
    if (!t || !t->wv) return JWE_ERR_BAD_TAB;
    // 今のページの出どころが完全に同じときだけ
    LPWSTR src = nullptr;
    t->wv->get_Source(&src);
    std::string now_url = take(src);
    char now[512];
    jwe_site_of(now_url.data(), now_url.size(), now, sizeof now);
    if (!now[0] || strlen(now) != site_len || memcmp(now, site, site_len) != 0) return JWE_ERR_BAD_ARG;
    // ページの中でも、入れる直前に出どころを確かめる（確かめてから入れるまでのあいだに移ったときのため）。
    // 見えているパスワードの欄（フォーカスがあればそれ）と、その前の文字の欄に入れ、input / change を起こす
    std::wstring script =
        L"(function(o,u,p){if(location.origin!==o)return 0;"
        L"var vis=function(x){return !x.disabled&&!x.readOnly&&x.offsetParent!==null;};"
        L"var pw=Array.prototype.filter.call(document.querySelectorAll('input[type=password]'),vis);"
        L"if(!pw.length)return 0;"
        L"var a=document.activeElement,P=(a&&a.type==='password'&&vis(a))?a:pw[0];"
        L"var set=function(x,v){var d=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value');"
        L"d.set.call(x,v);x.dispatchEvent(new Event('input',{bubbles:true}));x.dispatchEvent(new Event('change',{bubbles:true}));};"
        L"var all=Array.prototype.slice.call((P.form||document).querySelectorAll('input')),i=all.indexOf(P),U=null;"
        L"for(var k=i-1;k>=0;k--){var ty=(all[k].type||'text').toLowerCase();"
        L"if((ty==='text'||ty==='email'||ty==='tel')&&vis(all[k])){U=all[k];break;}}"
        L"if(U&&u)set(U,u);set(P,p);P.focus();return 1;})(" +
        js_string(widen(site, site_len)) + L"," + js_string(widen(user, user_len)) + L"," +
        js_string(widen(password, password_len)) + L")";
    HRESULT hr = t->wv->ExecuteScript(script.c_str(), new ScriptDone([](HRESULT, LPCWSTR) { return S_OK; }));
    // 書いた文字列は消しておく（パスワードを長く残さない）
    SecureZeroMemory(&script[0], script.size() * sizeof(wchar_t));
    return SUCCEEDED(hr) ? JWE_OK : JWE_ERR_FAILED;
}

JweStatus jwe_clear_site_data(JweEngine *e) {
    // どれか 1 つの WebView から DevTools の命令で消す（同じ環境のタブすべてに効く）
    ICoreWebView2 *wv = e->chrome;
    for (auto &t : e->tabs)
        if (!wv && t.wv) wv = t.wv;
    if (!wv) return JWE_ERR_FAILED;
    static const wchar_t *methods[] = {L"Network.clearBrowserCookies", L"Network.clearBrowserCache"};
    for (const wchar_t *m : methods)
        wv->CallDevToolsProtocolMethod(m, L"{}", new DevToolsDone([](HRESULT, LPCWSTR) { return S_OK; }));
    return JWE_OK;
}

} // extern "C"
