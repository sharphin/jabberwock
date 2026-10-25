# Windows向けGUI・描画APIの設計

状態: 実装前の詳細設計。以下の型・API・設定は提案であり、現行機能ではない。
Windows先行、ゲームにも使える描画基盤という合意に基づく。
[全体方針](standard-libraries.md)、[共通資源管理](resource-management.md)、[alice拡張](alice-native.md) と整合させる。
widget tree、配置、入力の消費、TSFとUI Automationの具体的な契約は [GUI内部仕様](gui-internals.md) に定める。
公開名・型・引数順は [公開API](public-apis.md) と [v1公開契約](stdlib-v1-contracts.md) を正本とする。

## 構成と初期範囲

| 層 | 初期の責任 |
|---|---|
| Window・イベント | Win32のウィンドウ、DPI、入力、閉じる要求、待ち・非ブロッキング取得 |
| Renderer・Frame | D3D11/DXGIによる2D描画、画像、クリップ、表示、デバイス喪失 |
| 文字 | DirectWriteによる配置、Direct2Dとの相互運用による描画 |
| GUI | 保持するwidget tree、レイアウト、フォーカス、ボタン、ラベル、入力欄 |
| アプリ | イベントの解釈、ゲーム状態・業務状態の更新、I/O workerとの連携 |

ゲームはWindow・入力・Rendererだけを使える。GUIは同じFrameへ描画でき、アプリのループを独占しない。
初期はWindows 10 1809以降/Windows 11 x64とD3D feature level 11.0以上を想定する。
Linuxの実描画、3Dの公開API、任意shader、音声、動画、タッチ・ゲームパッドは後の範囲とする。
Linuxの公開APIはここで確定せず、Windows固有の型を公開しないことで別backendを接続できる構造を保つ。

## 公開APIの草案

| API | 契約 |
|---|---|
| `create_window(options)` | `Result<Window, WindowError>`。title、論理サイズ、サイズ変更可否など |
| `Window.poll_events()` | 待たずに所有するEventBatchを返す。GUI資源の作成スレッドで呼ぶ |
| `Window.wait_events(deadline)` | イベントまたは期限まで待つ。無限のbusy loopにしない |
| `Window.info()` | 論理サイズ、物理描画サイズ、DPI倍率、フォーカス、最小化状態の値 |
| `Window.request_redraw()` | 再描画の要求。即座にユーザーコードを呼び返さない |
| `Window.close()` | 所有値を消費する。依存する描画を停止・無効化してから閉じる |
| `create_renderer(window, options)` | `Result<Renderer, DrawError>`。Windowとの内部依存を記録 |
| `Renderer.close()` / `Texture.close()` | 所有値を消費し、依存と世代の規則に従って解放 |
| `create_text_layout(font, text, options)` | font設定と所有する文字からlayoutを作る。幅・サイズ・上限を検査 |
| `Renderer.info()` | 選んだadapterの種類、feature level、描画上限などの値 |
| `Renderer.begin_frame(clear_color)` | ReadyならFrame、最小化などで描画できないならSkipped |
| `Frame.fill_rect` / `stroke_rect` / `triangle` | 検査済みの2D描画命令を蓄積する |
| `Frame.image` / `text` | Texture・文字layoutとclip/transformを使う |
| `Frame.push_clip` / `pop_clip` / `push_transform` / `pop_transform` | 深さと対応を検査する |
| `Frame.finish(present_options)` | Frameを消費して送信・表示する。Rendererを次のフレームへ戻す |
| `Frame.abort()` | Frameを消費し、蓄積した処理と内部leaseを解放する |
| `Texture.from_rgba8(renderer, size, pixels, options)` | 検査した画像を所有するTextureにする |
| `create_gui(window, options)` | GUIの状態とwidget treeを所有するGuiContext |
| `GuiContext.update(batch, now)` | batchを消費し、GUIイベントとゲーム用の残り入力、再描画・期限を返す |
| `GuiContext.close()` | GUIの所有値を消費。Window自体を閉じず、IME・focus・native連携を解除 |
| `GuiContext.draw(frame)` | 同じWindowに属するFrameへGUIを描く |

Window/Draw/Guiのエラー型を分ける。その他の草案表記も公開API表のResultと返却型に従う。
D3Dのdevice/context、COMポインタ、HWND、OSメッセージ番号は公開APIに含めない。

## スレッドとイベントの寿命

GUIを初期化してWindowを作るスレッドをGUIスレッドとする。OSのプロセス開始スレッドとの同一性は要求しない。
Window・Renderer・Frame・Texture・GuiContext・文字layoutは初期段階では送信不可にする。
COMが必要な機能は同じスレッドで初期化・終了し、自分が取得していない初期化を解除しない。

Window procedureはOSの状態を更新してイベントを蓄積するだけとし、描画や任意のユーザー関数を直接呼ばない。
WM_PAINTもOS側のpaint処理を完結させてRedrawRequestedへ変換し、user callbackで再入させない。
返すイベントはbatch内の所有する値とし、OSの一時バッファへのrefやポインタを含めない。

初期イベントはCloseRequested、Resized、DpiChanged、FocusChanged、RedrawRequested、
KeyDown/Up、PointerMove/Button/Wheel、TextInput、Compositionを候補とする。
物理的なキー、リピート、文字入力を区別し、キーボード配列から文字を独自に推測しない。
フォーカスを失ったときは押下・capture・IME状態を解放し、キーが押されたままにならないようにする。

イベント列に上限を設ける。pointer移動・resizeなど同等の通知は規則を決めてまとめられるが、
文字・押下/解放・閉じる要求を黙って捨てない。上限超過はInputReset/Overflowとして検出し、
入力状態を一貫した状態へ戻す。閉じる要求などの重要状態は別に保持し、queueが満杯でも失わない。

CloseRequestedは閉じるかどうかをアプリが判断するイベント。取得した時点でWindowを解放しない。
複数Windowは独立したイベントと資源世代を持ち、1つを閉じても他のWindowを無効にしない。

## 座標・色・画像

公開2D座標は左上が原点、右と下を正とする論理ピクセルを既定とする。
Window.infoから物理描画サイズとDPI倍率を取得できる。ゲームが物理ピクセルで描く場合は、明示的な座標系変換を使う。
入力、hit test、描画、clip、文字配置の変換をそろえる。OSが要求する画面座標と混同しない。

座標・変換・時間などのf64は有限性を検査する。幅・高さ・線幅は非負、空の矩形は描画なしとして扱う。
NaNや無限大をGPU APIや確保サイズへそのまま渡さない。
色は公開側ではsRGBのRGBA値とし、内部の形式・alphaの乗算・blend規則を定義して境界で変換する。

Textureの寸法、幅×高さ×pixelサイズ、stride、入力長、deviceの上限を確保前に検査する。
最初の画像形式はRGBA8とし、Nearest/Linearのsamplingを指定できるようにする。
CPU側のpixel領域は必要な転送が終わるまで保持し、ユーザーの可変バッファへのポインタを蓄積しない。

符号化された画像の読み込みは追加のimage能力として分ける案とする。
WindowsのWICなどを使い、最初はPNG/JPEGの単一画像に範囲を絞る。
復号したCPU画像は寸法と所有するpixel列を持つ値としてworkerから送れるようにし、TextureはGUIスレッドで作る。
復号前・復号中の寸法とメモリ上限を扱い、圧縮ファイルの小ささだけで安全な確保量とは判断しない。
image能力を未実装のまま、encoded画像が読めると説明しない。

## Frameと資源の状態

RendererはReady、FrameOpen、SurfaceUnavailable、DeviceLost、Closedを区別する。
begin_frameで内部の描画leaseをFrameへ渡し、同じRendererで2つ目のFrameを始めない。
ref mutが排他的でないことを前提に、runtimeでFrameの所有・世代・所属Rendererを確認する。

Frameに蓄積した命令は、参照するTexture・文字layoutなどの内部leaseを保持する。
所有者が途中で閉じられても解放済みpayloadを参照しない。失効した命令は明確に拒否する。
命令数・clip/transformの深さ・一時メモリ・画像転送量に上限を持つ。
描画順を保ち、alphaやclipの結果が変わるように命令を勝手に並べ替えない。

finishはstackの対応、命令、資源を確認してから処理する。失敗したFrameを成功としてPresentしない。
Frameを捨てた場合はabortとして片づける。dropから暗黙のPresentはしない。
GPUへ渡した資源はD3Dの寿命規則に従い、contextのbindingや一時領域を適切に解放する。
CPUの命令列を破棄できたこととGPUが処理を完了したことを混同しない。

Window.closeはFrameを無効化し、GUIのnative連携、swap chain・描画先などを依存の逆順に回収してからWindowを破棄する。
残った所有値は失効した資源となり、その後のdropで二重に解放しない。
Rendererに属するTextureは別Rendererへ渡せず、Rendererを作り直すと旧世代のTextureは使えない。

## D3D11・文字描画・デバイス再作成

初期のadapter選択はHardwarePreferred、HardwareRequired、Warpを候補とする。
HardwarePreferredでfallbackした場合はRenderer.infoに示す。HardwareRequiredを黙ってWARPへ変えない。
Windows CIの描画検証はWARPを明示して使い、hardwareで動いたという証拠にはしない。

Direct2Dとの相互運用に必要な形式・BGRA対応・DXGI surfaceを確認する。
D3DからDirect2Dへ移る順、bindingの解除、flush、EndDraw、Presentをbackendが管理する。
DirectWriteだけで文字を描画できたとは扱わず、実際の描画先への出力とpixel読み戻しを検証する。
任意のinterfaceを確認せずcastせず、COMの取得・失敗・解放を対応させる。

サイズ0・最小化では描画をSkippedとし、0サイズでswap chainを作り直し続けない。
resizeでは古い描画先のbindingとleaseを整理してから新しい描画先を作る。
デバイス喪失ではDeviceLostを返し、旧Frameとdevice依存資源を失効させる。
最初は明示的にRendererと必要なTextureを再作成し、保存していないpixelからTextureを復元できると約束しない。

GuiContextのwidget状態はWindowに属し、特定Rendererを常に所有するものにはしない。
同じWindowの新しいRendererへdrawできるようにし、デバイス再作成で入力中の文章まで無条件に失わない設計にする。
文字layoutのCPU側情報と、device依存の描画用情報も区別する。

## widget treeとアプリのイベント

初期GUIは保持するwidget treeとする。Windowごとに一つのGuiContextを持つ。
NodeIdはGuiContextの同一性と世代を持ち、削除したnode・異なるcontext・再利用した番号を検出する。
作成・親子付け・削除で循環を許さず、深さ・node数・文字サイズを上限で検査する。

初期部品はPanel、Row、Column、Label、Button、単一行TextInputとする。
最小/最大サイズ・余白・配置を持つ小さいlayoutから始め、CSS互換を目標にしない。
APIはnodeの作成、文字・状態・layoutの設定、削除、focusの要求を提供する。
削除は子のNodeId、focus、capture、compositionも整合した状態へ戻す。

GuiContext.updateは入力を処理し、Clicked、TextChanged、Submitted、FocusChangedなどの所有するイベントを返す。
ユーザーの処理はupdateの後に行い、treeを変更する。layout中・native解放中にアプリの関数を呼び返さない。
workerのI/O結果もこの安全な時点で適用する。
GUIが消費した入力と、ゲーム側へ渡す入力を区別し、入力欄に入力中のキーがゲーム操作にも二重適用されないようにする。

## 日本語入力・文字の境界・アクセシビリティ

TextInputは確定した文字列と未確定のcompositionを分け、キャンセルした未確定文字を確定値へ混ぜない。
UTF-8/UTF-16の対応を管理し、surrogateやUTF-8 byteの途中へカーソルを置かない。
caret・選択・hit testはOSの文字処理とDirectWriteのlayoutに合わせ、独自のbyte単位の処理で代用しない。
日本語、結合文字、emoji、双方向文字、複数行の対応範囲を実際に試験して記録する。

IMEはTSFを用い、候補位置、composition、確定、focus移動、編集途中の削除、clipboardの所有と文字コードを扱う。
clipboardを開いたままにしない。取得したOS領域を解放・unlockした後にその領域へのrefを返さない。

自前描画の部品にはWindows UI Automationへの対応が必要になる。
role・名前・状態・focus・画面上の範囲をsemantic treeとして保持する。
別スレッドからの照会は安全なsnapshotを参照し、操作要求はGUIスレッドへ渡す。
表の錠を持ったままGUI threadへの同期呼び出しを待ち、相互に待ち合う設計にしない。
Windowが閉じた後の問い合わせは、解放済みのnodeへ触れず利用不可として返す。

矩形と文字の描画だけをGUI完成とは判定しない。日本語の入力欄を対応済みとするにはIMEとclipboard、
標準的なGUI部品を対応済みとするにはkeyboard操作・focus・支援技術からの照会と操作も確認する。

## ループとI/Oの統合

ゲームの基本手順は、poll、GUI入力処理、アプリ状態の更新、begin_frame、ゲーム描画、GUI描画、finishとする。
GUI中心のアプリは、変更がない間wait_eventsを使い、通知が来たときだけ描画する。
時間は単調時計から取得し、フレーム数と経過時間を同一視しない。
共通のClock.now/elapsedとDuration/Deadlineを使い、NaNや異なる時計の値を混ぜない。

ファイル・ネットワーク・DBの長い操作はworkerで行う。WindowやRendererをworkerへ送らず、所有する値だけを返す。
通常の実行でデバッグ照合のためにWindowやDB操作を複数回実行しない。
[共通資源管理](resource-management.md) の外部副作用の規則を適用する。

## 完了条件

| 段階 | 検証 |
|---|---|
| Window・入力 | 作成/破棄反復、複数Window、close要求の取消し、DPI・focus・最小化、queue上限と入力reset |
| 2D描画 | WARPで図形・alpha・clip・変換・画像・文字を描き、pixel読み戻しで確認 |
| 所有・失敗 | 別thread・別device・旧Frame・親のclose・途中失敗、COMとOS資源の回収を検査 |
| device | resizeの反復、サイズ0、喪失注入、再作成後の旧Texture拒否、GUIの状態保持 |
| widget | layout、循環拒否、ID再利用、mouse/keyboard、focus/capture、ゲームとの入力消費の分離 |
| 文字・GUI実機 | 日本語IME・候補位置・選択・clipboard・DPIの移動、keyboardのみの操作、UI Automation |
| 画像 | 寸法・stride・入力長・上限、不正なPNG/JPEG、復号と転送の失敗、workerからのCPU画像 |
| 実行器と配布 | Rust/IR/ネイティブ/jwc、SDK不足、移動した配布物での起動、不要なDLLを要求しない |
| 安全性1〜7 | 模擬backendで固定攻撃・生成入力・sanitizerを実行し、Windows実APIも別に検証 |

CIのhidden windowやWARPによる成功と、人が操作する画面・実hardwareの確認を別々に記録する。
模擬backendだけで実際のIMEやGPU driverまで検証済みとはしない。この文書は機能の実装完了報告ではない。
