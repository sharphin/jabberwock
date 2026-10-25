# 標準ライブラリ公開API v1

状態: 実装前の契約。[registry.json](stdlib-v1/registry.json) から生成する。機能の提供・試験成功を意味しない。

型式のlist<T>/array<T,N>/option<T>/result<T,E>はそれぞれT[]/T[N]/Option<T>/Result<T,E>に対応する契約表記。
valueは複製可能な値、moveは値渡し（Copy型なら既存規則どおりコピー）、ref/mutは読取/可変借用。資源は移すか借り、複製しない。
default factoryは明記した値だけを生成し、未指定の接続情報や秘密を補完しない。型名・field名・引数順・返却型の正本はJSON。

[意味・既定値・上限](stdlib-v1-contracts.md)、[C境界](native-operations.md)、[登録と配布](catalog-registry.md) を併せて適用する。

## std.fs

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `Path.from_text` | `ref text: str` | `result<std.fs.Path,std.fs.IoError>` |
| `Path.to_text` | `ref self: std.fs.Path` | `result<str,std.fs.IoError>` |
| `Path.display` | `ref self: std.fs.Path` | `str` |
| `Path.file_name` | `ref self: std.fs.Path` | `option<std.fs.Path>` |
| `Path.join` | `ref self: std.fs.Path, value child: std.fs.Path` | `result<std.fs.Path,std.fs.IoError>` |
| `Path.absolute` | `ref self: std.fs.Path` | `result<std.fs.Path,std.fs.IoError>` |
| `Path.canonical` | `ref self: std.fs.Path` | `result<std.fs.Path,std.fs.IoError>` |
| `WindowsShare.defaults` | `なし` | `std.fs.WindowsShare` |
| `OpenOptions.defaults` | `なし` | `std.fs.OpenOptions` |
| `open` | `value path: std.fs.Path, move options: std.fs.OpenOptions` | `result<std.fs.File,std.fs.IoError>` |
| `read_bytes` | `value path: std.fs.Path, value limit: u64` | `result<list<u8>,std.fs.IoError>` |
| `read_text` | `value path: std.fs.Path, value limit: u64` | `result<str,std.fs.IoError>` |
| `read_dir` | `value path: std.fs.Path, value limit: u64` | `result<list<std.fs.Entry>,std.fs.IoError>` |
| `write_bytes` | `value path: std.fs.Path, ref data: list<u8>, move mode: std.fs.WriteMode` | `result<unit,std.fs.WriteFailure>` |
| `metadata` | `value path: std.fs.Path, value follow: std.fs.Follow` | `result<std.fs.Metadata,std.fs.IoError>` |
| `create_dir` | `value path: std.fs.Path` | `result<unit,std.fs.IoError>` |
| `remove_file` | `value path: std.fs.Path` | `result<unit,std.fs.IoError>` |
| `remove_empty_dir` | `value path: std.fs.Path` | `result<unit,std.fs.IoError>` |
| `rename` | `value source: std.fs.Path, value target: std.fs.Path, value replace: std.fs.Replace` | `result<unit,std.fs.IoError>` |
| `File.read` | `mut self: std.fs.File, value max_bytes: u64` | `result<std.fs.ReadChunk,std.fs.IoError>` |
| `File.write` | `mut self: std.fs.File, ref data: list<u8>` | `result<u64,std.fs.WriteFailure>` |
| `File.write_all` | `mut self: std.fs.File, ref data: list<u8>` | `result<unit,std.fs.WriteFailure>` |
| `File.seek` | `mut self: std.fs.File, value offset: i64, value origin: std.fs.SeekOrigin` | `result<u64,std.fs.IoError>` |
| `File.length` | `ref self: std.fs.File` | `result<u64,std.fs.IoError>` |
| `File.metadata` | `ref self: std.fs.File` | `result<std.fs.Metadata,std.fs.IoError>` |
| `File.set_length` | `mut self: std.fs.File, value length: u64` | `result<unit,std.fs.IoError>` |
| `File.flush` | `mut self: std.fs.File` | `result<unit,std.fs.IoError>` |
| `File.sync` | `mut self: std.fs.File` | `result<unit,std.fs.IoError>` |
| `File.close` | `move self: std.fs.File` | `result<unit,std.fs.IoError>` |

| 型 | 定義 |
|---|---|
| `std.fs.Path` | `opaque { encoding: u32, bytes: list<u8> } Copy=True, Send=True` |
| `std.fs.File` | `resource kind 1; !Copy/!Send` |
| `std.fs.IoErrorKind` | `NotFound() [tag 0]; AlreadyExists() [tag 1]; PermissionDenied() [tag 2]; InvalidPath() [tag 3]; InvalidEncoding() [tag 4]; WrongKind() [tag 5]; Closed() [tag 6]; Busy() [tag 7]; Unsupported() [tag 8]; LimitExceeded() [tag 9]; NoProgress() [tag 10]; InvalidArgument() [tag 11]; OsError() [tag 12]` |
| `std.fs.IoError` | `record { kind: std.fs.IoErrorKind, operation: str, os_code: option<i64> }` |
| `std.fs.Creation` | `Existing() [tag 0]; Create() [tag 1]; NewOnly() [tag 2]; Truncate() [tag 3]; CreateOrTruncate() [tag 4]` |
| `std.fs.SeekOrigin` | `Start() [tag 0]; Current() [tag 1]; End() [tag 2]` |
| `std.fs.Replace` | `NoReplace() [tag 0]; Replace() [tag 1]` |
| `std.fs.Follow` | `NoFollow() [tag 0]; Follow() [tag 1]` |
| `std.fs.FileKind` | `Regular() [tag 0]; Directory() [tag 1]; Link() [tag 2]; Other() [tag 3]` |
| `std.fs.WindowsShare` | `record { read: bool, write: bool, delete: bool }; defaults={"read":true,"write":true,"delete":true}` |
| `std.fs.OpenOptions` | `record { read: bool, write: bool, creation: std.fs.Creation, append: bool, windows_share: option<std.fs.WindowsShare> }; defaults={"read":true,"write":false,"creation":{"variant":"Existing","fields":{}},"append":false,"windows_share":null}` |
| `std.fs.WriteMode` | `record { creation: std.fs.Creation, append: bool, windows_share: option<std.fs.WindowsShare> }` |
| `std.fs.UnixTime` | `record { seconds: i64, nanoseconds: u32 }` |
| `std.fs.Metadata` | `record { kind: std.fs.FileKind, length: u64, read_only: bool, modified: option<std.fs.UnixTime> }` |
| `std.fs.Entry` | `record { name: std.fs.Path, path: std.fs.Path, kind: option<std.fs.FileKind> }` |
| `std.fs.ReadChunk` | `Data(bytes: list<u8>) [tag 0]; Eof() [tag 1]` |
| `std.fs.WriteFailure` | `record { error: std.fs.IoError, written: u64, may_have_written_more: bool, target_may_have_changed: bool }` |

## std.time

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `now` | `なし` | `result<std.time.Instant,std.time.TimeError>` |
| `elapsed` | `value since: std.time.Instant` | `result<std.time.Duration,std.time.TimeError>` |
| `Duration.from_millis` | `value value: u64` | `result<std.time.Duration,std.time.TimeError>` |
| `Duration.from_seconds` | `value value: u64` | `result<std.time.Duration,std.time.TimeError>` |
| `Duration.from_nanos` | `value value: u64` | `std.time.Duration` |
| `Duration.nanos` | `ref self: std.time.Duration` | `u64` |
| `Deadline.infinite` | `なし` | `std.time.Deadline` |
| `deadline_after` | `value duration: std.time.Duration` | `result<std.time.Deadline,std.time.TimeError>` |
| `deadline_at` | `value instant: std.time.Instant` | `result<std.time.Deadline,std.time.TimeError>` |

| 型 | 定義 |
|---|---|
| `std.time.Duration` | `opaque { nanoseconds: u64 } Copy=True, Send=True` |
| `std.time.Instant` | `opaque { clock_id: u64, nanoseconds: u64 } Copy=True, Send=True` |
| `std.time.Deadline` | `opaque { clock_id: u64, nanoseconds: u64, mode: u32, reserved: u32 } Copy=True, Send=True` |
| `std.time.TimeErrorKind` | `InvalidArgument() [tag 0]; Overflow() [tag 1]; WrongClock() [tag 2]; NativeError() [tag 3]` |
| `std.time.TimeError` | `record { kind: std.time.TimeErrorKind, operation: str, os_code: option<i64> }` |

## std.net

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `IpAddress.v4` | `ref bytes: array<u8,4>` | `std.net.IpAddress` |
| `IpAddress.v6` | `ref bytes: array<u8,16>` | `std.net.IpAddress` |
| `SocketAddress.create` | `value ip: std.net.IpAddress, value port: u16, value scope: u32` | `result<std.net.SocketAddress,std.net.NetError>` |
| `SocketAddress.ip` | `ref self: std.net.SocketAddress` | `std.net.IpAddress` |
| `SocketAddress.port` | `ref self: std.net.SocketAddress` | `u16` |
| `SocketAddress.scope` | `ref self: std.net.SocketAddress` | `u32` |
| `BindOptions.defaults` | `なし` | `std.net.BindOptions` |
| `resolve` | `ref host: str, value port: u16, value deadline: std.time.Deadline` | `result<list<std.net.SocketAddress>,std.net.NetError>` |
| `connect` | `move target: std.net.Target, value deadline: std.time.Deadline` | `result<std.net.TcpStream,std.net.NetError>` |
| `bind` | `value address: std.net.SocketAddress, move options: std.net.BindOptions` | `result<std.net.TcpListener,std.net.NetError>` |
| `TcpListener.accept` | `mut self: std.net.TcpListener, value deadline: std.time.Deadline` | `result<std.net.Accepted,std.net.NetError>` |
| `TcpListener.address` | `ref self: std.net.TcpListener` | `result<std.net.SocketAddress,std.net.NetError>` |
| `TcpListener.close` | `move self: std.net.TcpListener` | `result<unit,std.net.NetError>` |
| `TcpStream.read` | `mut self: std.net.TcpStream, value max_bytes: u64, value deadline: std.time.Deadline` | `result<std.net.ReadChunk,std.net.NetError>` |
| `TcpStream.read_exact` | `mut self: std.net.TcpStream, value bytes: u64, value deadline: std.time.Deadline` | `result<list<u8>,std.net.ReadFailure>` |
| `TcpStream.write` | `mut self: std.net.TcpStream, ref data: list<u8>, value deadline: std.time.Deadline` | `result<u64,std.net.WriteFailure>` |
| `TcpStream.write_all` | `mut self: std.net.TcpStream, ref data: list<u8>, value deadline: std.time.Deadline` | `result<unit,std.net.WriteFailure>` |
| `TcpStream.local_address` | `ref self: std.net.TcpStream` | `result<std.net.SocketAddress,std.net.NetError>` |
| `TcpStream.peer_address` | `ref self: std.net.TcpStream` | `result<std.net.SocketAddress,std.net.NetError>` |
| `TcpStream.shutdown_write` | `mut self: std.net.TcpStream` | `result<unit,std.net.NetError>` |
| `TcpStream.close` | `move self: std.net.TcpStream` | `result<unit,std.net.NetError>` |

| 型 | 定義 |
|---|---|
| `std.net.IpAddress` | `opaque { family: u32, address: array<u8,16> } Copy=True, Send=True` |
| `std.net.SocketAddress` | `opaque { family: u32, port: u16, reserved: u16, address: array<u8,16>, scope: u32, reserved2: u32 } Copy=True, Send=True` |
| `std.net.TcpListener` | `resource kind 2; !Copy/!Send` |
| `std.net.TcpStream` | `resource kind 3; !Copy/!Send` |
| `std.net.NetErrorKind` | `Resolve() [tag 0]; Refused() [tag 1]; Timeout() [tag 2]; PeerReset() [tag 3]; UnexpectedEof() [tag 4]; Tls() [tag 5]; InvalidArgument() [tag 6]; Closed() [tag 7]; Busy() [tag 8]; LimitExceeded() [tag 9]; NoProgress() [tag 10]; Unsupported() [tag 11]; OsError() [tag 12]` |
| `std.net.NetError` | `record { kind: std.net.NetErrorKind, operation: str, os_code: option<i64> }` |
| `std.net.Target` | `Address(address: std.net.SocketAddress) [tag 0]; Host(host: str, port: u16) [tag 1]` |
| `std.net.BindOptions` | `record { backlog: u32, reuse_address: bool, ipv6_only: bool }; defaults={"backlog":128,"reuse_address":false,"ipv6_only":true}` |
| `std.net.Accepted` | `record { stream: std.net.TcpStream, peer: std.net.SocketAddress }` |
| `std.net.ReadFailure` | `record { error: std.net.NetError, partial: list<u8> }` |
| `std.net.WriteFailure` | `record { error: std.net.NetError, written: u64, may_have_written_more: bool }` |
| `std.net.ReadChunk` | `Data(bytes: list<u8>) [tag 0]; Eof() [tag 1]` |

## std.tls

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `Options.defaults` | `なし` | `std.tls.Options` |
| `connect` | `move target: std.net.Target, move name: std.tls.ServerName, move options: std.tls.Options, value deadline: std.time.Deadline` | `result<std.tls.TlsStream,std.net.NetError>` |
| `start_tls` | `move stream: std.net.TcpStream, move name: std.tls.ServerName, move options: std.tls.Options, value deadline: std.time.Deadline` | `result<std.tls.TlsStream,std.net.NetError>` |
| `TlsStream.read` | `mut self: std.tls.TlsStream, value max_bytes: u64, value deadline: std.time.Deadline` | `result<std.net.ReadChunk,std.net.NetError>` |
| `TlsStream.write` | `mut self: std.tls.TlsStream, ref data: list<u8>, value deadline: std.time.Deadline` | `result<u64,std.net.WriteFailure>` |
| `TlsStream.write_all` | `mut self: std.tls.TlsStream, ref data: list<u8>, value deadline: std.time.Deadline` | `result<unit,std.net.WriteFailure>` |
| `TlsStream.close` | `move self: std.tls.TlsStream, value deadline: std.time.Deadline` | `result<unit,std.net.NetError>` |

| 型 | 定義 |
|---|---|
| `std.tls.TlsStream` | `resource kind 4; !Copy/!Send` |
| `std.tls.ServerName` | `Dns(name: str) [tag 0]; Ip(address: std.net.IpAddress) [tag 1]` |
| `std.tls.MinimumVersion` | `Tls12() [tag 0]; Tls13() [tag 1]` |
| `std.tls.Options` | `record { minimum: std.tls.MinimumVersion }; defaults={"minimum":{"variant":"Tls12","fields":{}}}` |

## postgres

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `connect` | `move options: postgres.ConnectOptions` | `result<postgres.Connection,postgres.PgError>` |
| `QueryOptions.defaults` | `value deadline: std.time.Deadline` | `postgres.QueryOptions` |
| `TxOptions.defaults` | `value deadline: std.time.Deadline` | `postgres.TxOptions` |
| `Connection.run` | `mut self: postgres.Connection, ref sql: str, ref params: list<postgres.Param>, move options: postgres.QueryOptions` | `result<postgres.QueryResult,postgres.PgError>` |
| `Transaction.run` | `mut self: postgres.Transaction, ref sql: str, ref params: list<postgres.Param>, move options: postgres.QueryOptions` | `result<postgres.QueryResult,postgres.PgError>` |
| `Connection.begin` | `move self: postgres.Connection, move options: postgres.TxOptions` | `result<postgres.Transaction,postgres.BeginFailure>` |
| `Connection.close` | `move self: postgres.Connection` | `result<unit,postgres.PgError>` |
| `Transaction.commit` | `move self: postgres.Transaction, value deadline: std.time.Deadline` | `postgres.TxEnd` |
| `Transaction.rollback` | `move self: postgres.Transaction, value deadline: std.time.Deadline` | `postgres.TxEnd` |
| `QueryResult.column_index` | `ref self: postgres.QueryResult, ref name: str` | `result<u32,postgres.PgError>` |
| `Row.cell` | `ref self: postgres.Row, value index: u32` | `result<postgres.Cell,postgres.PgError>` |
| `Cell.as_bool` | `ref self: postgres.Cell` | `result<bool,postgres.PgError>` |
| `Cell.as_i64` | `ref self: postgres.Cell` | `result<i64,postgres.PgError>` |
| `Cell.as_f64` | `ref self: postgres.Cell` | `result<f64,postgres.PgError>` |
| `Cell.as_text` | `ref self: postgres.Cell` | `result<str,postgres.PgError>` |
| `Cell.as_bytes` | `ref self: postgres.Cell` | `result<list<u8>,postgres.PgError>` |

| 型 | 定義 |
|---|---|
| `postgres.Connection` | `resource kind 5; !Copy/!Send` |
| `postgres.Transaction` | `resource kind 6; !Copy/!Send` |
| `postgres.Execution` | `NotSent() [tag 0]; MayHaveRun() [tag 1]; Completed() [tag 2]` |
| `postgres.PgErrorKind` | `Configuration() [tag 0]; Connect() [tag 1]; Authentication() [tag 2]; Tls() [tag 3]; Timeout() [tag 4]; Server() [tag 5]; Type() [tag 6]; LimitExceeded() [tag 7]; Closed() [tag 8]; Busy() [tag 9]; InvalidArgument() [tag 10]; Unsupported() [tag 11]; Transport() [tag 12]; Protocol() [tag 13]; NotFound() [tag 14]; Ambiguous() [tag 15]` |
| `postgres.PgError` | `record { kind: postgres.PgErrorKind, operation: str, os_code: option<i64>, sqlstate: option<str>, execution: postgres.Execution }` |
| `postgres.Security` | `VerifyFull(ca_file: std.fs.Path) [tag 0]; LoopbackPlain() [tag 1]` |
| `postgres.ConnectOptions` | `record { host: str, port: u16, database: str, user: str, password: str, security: postgres.Security, deadline: std.time.Deadline }` |
| `postgres.QueryOptions` | `record { deadline: std.time.Deadline, max_rows: u64, max_columns: u32, max_bytes: u64 }` |
| `postgres.Isolation` | `ReadCommitted() [tag 0]; RepeatableRead() [tag 1]; Serializable() [tag 2]` |
| `postgres.TxOptions` | `record { isolation: postgres.Isolation, read_only: bool, deadline: std.time.Deadline }` |
| `postgres.Param` | `Null(oid: option<u32>) [tag 0]; Bool(value: bool) [tag 1]; I64(value: i64) [tag 2]; F64(value: f64) [tag 3]; Text(value: str) [tag 4]; Bytes(value: list<u8>) [tag 5]` |
| `postgres.Cell` | `Null() [tag 0]; Text(value: str) [tag 1]; Bytes(value: list<u8>) [tag 2]` |
| `postgres.Column` | `record { name: str, oid: u32 }` |
| `postgres.Row` | `record { cells: list<postgres.Cell> }` |
| `postgres.Command` | `record { tag: str, affected_rows: option<u64> }` |
| `postgres.QueryResult` | `Command(command: postgres.Command) [tag 0]; Rows(columns: list<postgres.Column>, rows: list<postgres.Row>, command: postgres.Command) [tag 1]` |
| `postgres.BeginFailure` | `record { error: postgres.PgError, connection: option<postgres.Connection> }` |
| `postgres.TxOutcome` | `Committed() [tag 0]; RolledBack() [tag 1]; Unknown() [tag 2]` |
| `postgres.TxEnd` | `record { outcome: postgres.TxOutcome, connection: option<postgres.Connection>, error: option<postgres.PgError> }` |

## window

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `Options.defaults` | `なし` | `window.Options` |
| `create_window` | `move options: window.Options` | `result<window.Window,window.WindowError>` |
| `Window.poll_events` | `mut self: window.Window` | `result<window.EventBatch,window.WindowError>` |
| `Window.wait_events` | `mut self: window.Window, value deadline: std.time.Deadline` | `result<window.EventBatch,window.WindowError>` |
| `Window.info` | `ref self: window.Window` | `result<window.Info,window.WindowError>` |
| `Window.request_redraw` | `mut self: window.Window` | `result<unit,window.WindowError>` |
| `Window.close` | `move self: window.Window` | `result<unit,window.WindowError>` |
| `EventBatch.events` | `ref self: window.EventBatch` | `list<window.EventRecord>` |
| `GameInputBatch.events` | `ref self: window.GameInputBatch` | `list<window.EventRecord>` |

| 型 | 定義 |
|---|---|
| `window.Window` | `resource kind 7; !Copy/!Send` |
| `window.WindowErrorKind` | `InvalidArgument() [tag 0]; LimitExceeded() [tag 1]; Closed() [tag 2]; Busy() [tag 3]; Unsupported() [tag 4]; NativeError() [tag 5]` |
| `window.WindowError` | `record { kind: window.WindowErrorKind, operation: str, os_code: option<i64> }` |
| `window.Point` | `record { x: f64, y: f64 }` |
| `window.Size` | `record { width: f64, height: f64 }` |
| `window.PixelSize` | `record { width: u32, height: u32 }` |
| `window.Viewport` | `record { logical: window.Size, physical: window.PixelSize, dpi: f64, revision: u64 }` |
| `window.WindowId` | `opaque { slot: u64, generation: u64 } Copy=True, Send=True` |
| `window.Options` | `record { title: str, size: window.Size, resizable: bool }; defaults={"title":"","size":{"width":1280.0,"height":720.0},"resizable":true}` |
| `window.Info` | `record { id: window.WindowId, viewport: window.Viewport, focused: bool, minimized: bool, visible: bool, screen_x: i64, screen_y: i64 }` |
| `window.Key` | `Unknown() [tag 0]; Tab() [tag 1]; Enter() [tag 2]; Space() [tag 3]; Escape() [tag 4]; Backspace() [tag 5]; Delete() [tag 6]; Insert() [tag 7]; Left() [tag 8]; Right() [tag 9]; Up() [tag 10]; Down() [tag 11]; Home() [tag 12]; End() [tag 13]; PageUp() [tag 14]; PageDown() [tag 15]; Shift() [tag 16]; Control() [tag 17]; Alt() [tag 18]; Meta() [tag 19]; CapsLock() [tag 20]; NumLock() [tag 21]; ScrollLock() [tag 22]; Pause() [tag 23]; PrintScreen() [tag 24]; KeyA() [tag 25]; KeyB() [tag 26]; KeyC() [tag 27]; KeyD() [tag 28]; KeyE() [tag 29]; KeyF() [tag 30]; KeyG() [tag 31]; KeyH() [tag 32]; KeyI() [tag 33]; KeyJ() [tag 34]; KeyK() [tag 35]; KeyL() [tag 36]; KeyM() [tag 37]; KeyN() [tag 38]; KeyO() [tag 39]; KeyP() [tag 40]; KeyQ() [tag 41]; KeyR() [tag 42]; KeyS() [tag 43]; KeyT() [tag 44]; KeyU() [tag 45]; KeyV() [tag 46]; KeyW() [tag 47]; KeyX() [tag 48]; KeyY() [tag 49]; KeyZ() [tag 50]; Digit0() [tag 51]; Digit1() [tag 52]; Digit2() [tag 53]; Digit3() [tag 54]; Digit4() [tag 55]; Digit5() [tag 56]; Digit6() [tag 57]; Digit7() [tag 58]; Digit8() [tag 59]; Digit9() [tag 60]; F1() [tag 61]; F2() [tag 62]; F3() [tag 63]; F4() [tag 64]; F5() [tag 65]; F6() [tag 66]; F7() [tag 67]; F8() [tag 68]; F9() [tag 69]; F10() [tag 70]; F11() [tag 71]; F12() [tag 72]; F13() [tag 73]; F14() [tag 74]; F15() [tag 75]; F16() [tag 76]; F17() [tag 77]; F18() [tag 78]; F19() [tag 79]; F20() [tag 80]; F21() [tag 81]; F22() [tag 82]; F23() [tag 83]; F24() [tag 84]` |
| `window.MouseButton` | `Left() [tag 0]; Right() [tag 1]; Middle() [tag 2]; X1() [tag 3]; X2() [tag 4]` |
| `window.ResetReason` | `FocusLost() [tag 0]; CaptureLost() [tag 1]; Overflow() [tag 2]; Gap() [tag 3]; GuiCaptureChanged() [tag 4]` |
| `window.CompositionPhase` | `Start() [tag 0]; Update() [tag 1]; Commit() [tag 2]; Cancel() [tag 3]` |
| `window.PointerAction` | `Move() [tag 0]; Down() [tag 1]; Up() [tag 2]; Wheel() [tag 3]` |
| `window.WheelUnit` | `Lines() [tag 0]; Pages() [tag 1]` |
| `window.KeyEvent` | `record { key: window.Key, scan_code: u32, down: bool, repeat: bool, modifiers: u32, native_handled: bool }` |
| `window.PointerEvent` | `record { action: window.PointerAction, button: option<window.MouseButton>, position: window.Point, wheel_x: f64, wheel_y: f64, wheel_unit: window.WheelUnit }` |
| `window.CompositionData` | `record { edit_id: u64, phase: window.CompositionPhase, text: str, start_utf16: u32, end_utf16: u32, cursor_utf16: u32, native_handled: bool }` |
| `window.Event` | `CloseRequested(request_id: u64) [tag 0]; Resized(viewport: window.Viewport) [tag 1]; DpiChanged(viewport: window.Viewport) [tag 2]; FocusChanged(focused: bool) [tag 3]; RedrawRequested() [tag 4]; Key(event: window.KeyEvent) [tag 5]; Pointer(event: window.PointerEvent) [tag 6]; TextInput(text: str, edit_id: option<u64>, native_handled: bool) [tag 7]; Composition(event: window.CompositionData) [tag 8]; InputReset(reason: window.ResetReason) [tag 9]` |
| `window.EventRecord` | `record { sequence: u64, timestamp_ns: u64, viewport_revision: u64, event: window.Event }` |
| `window.EventBatch` | `opaque { window: window.WindowId, batch_id: u64, first_sequence: u64, last_sequence: u64, viewport: window.Viewport, events: list<window.EventRecord> } Copy=False, Send=True` |
| `window.GameInputBatch` | `opaque { window: window.WindowId, batch_id: u64, first_sequence: u64, last_sequence: u64, viewport: window.Viewport, events: list<window.EventRecord> } Copy=False, Send=True` |

## draw2d

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `RendererOptions.defaults` | `なし` | `draw2d.RendererOptions` |
| `TextureOptions.defaults` | `なし` | `draw2d.TextureOptions` |
| `TextStyle.defaults` | `なし` | `draw2d.TextStyle` |
| `PresentOptions.defaults` | `なし` | `draw2d.PresentOptions` |
| `create_renderer` | `mut window: window.Window, move options: draw2d.RendererOptions` | `result<draw2d.Renderer,draw2d.DrawError>` |
| `create_text_layout` | `ref text: str, move options: draw2d.TextOptions` | `result<draw2d.TextLayout,draw2d.DrawError>` |
| `Texture.from_rgba8` | `mut renderer: draw2d.Renderer, move size: draw2d.PixelSize, ref pixels: list<u8>, move options: draw2d.TextureOptions` | `result<draw2d.Texture,draw2d.DrawError>` |
| `Renderer.info` | `ref self: draw2d.Renderer` | `result<draw2d.RendererInfo,draw2d.DrawError>` |
| `Renderer.begin_frame` | `mut self: draw2d.Renderer, move clear: draw2d.Color` | `result<draw2d.FrameStart,draw2d.DrawError>` |
| `Renderer.close` | `move self: draw2d.Renderer` | `result<unit,draw2d.DrawError>` |
| `Texture.close` | `move self: draw2d.Texture` | `result<unit,draw2d.DrawError>` |
| `TextLayout.close` | `move self: draw2d.TextLayout` | `result<unit,draw2d.DrawError>` |
| `TextLayout.info` | `ref self: draw2d.TextLayout` | `result<draw2d.TextInfo,draw2d.DrawError>` |
| `Frame.fill_rect` | `mut self: draw2d.Frame, move rect: draw2d.Rect, move color: draw2d.Color` | `result<unit,draw2d.DrawError>` |
| `Frame.stroke_rect` | `mut self: draw2d.Frame, move rect: draw2d.Rect, move color: draw2d.Color, value width: f64` | `result<unit,draw2d.DrawError>` |
| `Frame.triangle` | `mut self: draw2d.Frame, move a: draw2d.Point, move b: draw2d.Point, move c: draw2d.Point, move color: draw2d.Color` | `result<unit,draw2d.DrawError>` |
| `Frame.image` | `mut self: draw2d.Frame, ref texture: draw2d.Texture, move source: option<draw2d.Rect>, move target: draw2d.Rect, move tint: draw2d.Color` | `result<unit,draw2d.DrawError>` |
| `Frame.text` | `mut self: draw2d.Frame, ref layout: draw2d.TextLayout, move origin: draw2d.Point, move color: draw2d.Color` | `result<unit,draw2d.DrawError>` |
| `Frame.push_clip` | `mut self: draw2d.Frame, move rect: draw2d.Rect` | `result<unit,draw2d.DrawError>` |
| `Frame.pop_clip` | `mut self: draw2d.Frame` | `result<unit,draw2d.DrawError>` |
| `Frame.push_transform` | `mut self: draw2d.Frame, move transform: draw2d.Transform` | `result<unit,draw2d.DrawError>` |
| `Frame.pop_transform` | `mut self: draw2d.Frame` | `result<unit,draw2d.DrawError>` |
| `Frame.finish` | `move self: draw2d.Frame, move options: draw2d.PresentOptions` | `result<draw2d.PresentInfo,draw2d.DrawError>` |
| `Frame.abort` | `move self: draw2d.Frame` | `result<unit,draw2d.DrawError>` |

| 型 | 定義 |
|---|---|
| `draw2d.Renderer` | `resource kind 8; !Copy/!Send` |
| `draw2d.Frame` | `resource kind 9; !Copy/!Send` |
| `draw2d.Texture` | `resource kind 10; !Copy/!Send` |
| `draw2d.TextLayout` | `resource kind 11; !Copy/!Send` |
| `draw2d.DrawErrorKind` | `InvalidArgument() [tag 0]; LimitExceeded() [tag 1]; Closed() [tag 2]; Busy() [tag 3]; Unsupported() [tag 4]; WrongWindow() [tag 5]; WrongDevice() [tag 6]; DeviceLost() [tag 7]; SurfaceUnavailable() [tag 8]; NativeError() [tag 9]` |
| `draw2d.DrawError` | `record { kind: draw2d.DrawErrorKind, operation: str, os_code: option<i64> }` |
| `draw2d.Point` | `record { x: f64, y: f64 }` |
| `draw2d.Rect` | `record { x: f64, y: f64, width: f64, height: f64 }` |
| `draw2d.PixelSize` | `record { width: u32, height: u32 }` |
| `draw2d.Color` | `record { red: f64, green: f64, blue: f64, alpha: f64 }` |
| `draw2d.Transform` | `record { m11: f64, m12: f64, m21: f64, m22: f64, dx: f64, dy: f64 }` |
| `draw2d.Adapter` | `HardwarePreferred() [tag 0]; HardwareRequired() [tag 1]; Warp() [tag 2]` |
| `draw2d.Sampling` | `Nearest() [tag 0]; Linear() [tag 1]` |
| `draw2d.RendererOptions` | `record { adapter: draw2d.Adapter }; defaults={"adapter":{"variant":"HardwarePreferred","fields":{}}}` |
| `draw2d.RendererInfo` | `record { warp: bool, feature_level: u32, max_texture_size: u32, device_generation: u64 }` |
| `draw2d.TextureOptions` | `record { sampling: draw2d.Sampling, stride: option<u64>, max_bytes: u64 }; defaults={"sampling":{"variant":"Linear","fields":{}},"stride":null,"max_bytes":67108864}` |
| `draw2d.Wrap` | `NoWrap() [tag 0]; WordWrap() [tag 1]` |
| `draw2d.Weight` | `Normal() [tag 0]; Bold() [tag 1]` |
| `draw2d.FontFamily` | `SystemUi() [tag 0]; Named(family: str) [tag 1]` |
| `draw2d.TextStyle` | `record { family: draw2d.FontFamily, size: option<f64>, weight: draw2d.Weight, wrap: draw2d.Wrap }; defaults={"family":{"variant":"SystemUi","fields":{}},"size":null,"weight":{"variant":"Normal","fields":{}},"wrap":{"variant":"NoWrap","fields":{}}}` |
| `draw2d.TextOptions` | `record { style: draw2d.TextStyle, max_width: option<f64>, max_bytes: u64 }` |
| `draw2d.TextInfo` | `record { width: f64, height: f64, lines: u32 }` |
| `draw2d.SkipReason` | `Minimized() [tag 0]; ZeroSize() [tag 1]; Occluded() [tag 2]` |
| `draw2d.FrameStart` | `Ready(frame: draw2d.Frame) [tag 0]; Skipped(reason: draw2d.SkipReason) [tag 1]` |
| `draw2d.PresentOptions` | `record { vsync: bool }; defaults={"vsync":true}` |
| `draw2d.PresentState` | `Presented() [tag 0]; Skipped() [tag 1]; Occluded() [tag 2]` |
| `draw2d.PresentInfo` | `record { state: draw2d.PresentState }` |

## image

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `Limits.defaults` | `なし` | `image.Limits` |
| `decode` | `ref encoded: list<u8>, move limits: image.Limits` | `result<image.CpuImage,image.ImageError>` |
| `load` | `value path: std.fs.Path, move limits: image.Limits` | `result<image.CpuImage,image.ImageError>` |

| 型 | 定義 |
|---|---|
| `image.ImageErrorKind` | `InvalidInput() [tag 0]; Unsupported() [tag 1]; LimitExceeded() [tag 2]; Io() [tag 3]; NativeError() [tag 4]` |
| `image.ImageError` | `record { kind: image.ImageErrorKind, operation: str, os_code: option<i64> }` |
| `image.CpuImage` | `record { width: u32, height: u32, stride: u64, pixels: list<u8> }` |
| `image.Limits` | `record { max_encoded_bytes: u64, max_pixels: u64, max_decoded_bytes: u64 }; defaults={"max_encoded_bytes":16777216,"max_pixels":16777216,"max_decoded_bytes":67108864}` |

## gui

| 操作 | 引数（順序・権限・型） | 戻り値 |
|---|---|---|
| `Insets.defaults` | `なし` | `gui.Insets` |
| `Layout.defaults` | `なし` | `gui.Layout` |
| `Semantics.defaults` | `なし` | `gui.Semantics` |
| `TextInputOptions.defaults` | `なし` | `gui.TextInputOptions` |
| `GuiLimits.defaults` | `なし` | `gui.GuiLimits` |
| `GuiOptions.defaults` | `なし` | `gui.GuiOptions` |
| `create_gui` | `mut window: window.Window, move options: gui.GuiOptions` | `result<gui.GuiContext,gui.GuiError>` |
| `GuiContext.root` | `ref self: gui.GuiContext` | `result<gui.NodeId,gui.GuiError>` |
| `GuiContext.create` | `mut self: gui.GuiContext, value parent: gui.NodeId, move spec: gui.WidgetSpec, move layout: gui.Layout, move semantics: gui.Semantics` | `result<gui.NodeId,gui.GuiError>` |
| `GuiContext.reparent` | `mut self: gui.GuiContext, value node: gui.NodeId, value parent: gui.NodeId, value index: u32` | `result<unit,gui.GuiError>` |
| `GuiContext.remove` | `mut self: gui.GuiContext, value node: gui.NodeId` | `result<unit,gui.GuiError>` |
| `GuiContext.set_layout` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: gui.Layout` | `result<unit,gui.GuiError>` |
| `GuiContext.set_style` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: gui.Style` | `result<unit,gui.GuiError>` |
| `GuiContext.set_text_style` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: draw2d.TextStyle` | `result<unit,gui.GuiError>` |
| `GuiContext.set_input_options` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: gui.TextInputOptions` | `result<unit,gui.GuiError>` |
| `GuiContext.set_semantics` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: gui.Semantics` | `result<unit,gui.GuiError>` |
| `GuiContext.set_visibility` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: gui.Visibility` | `result<unit,gui.GuiError>` |
| `GuiContext.set_selection` | `mut self: gui.GuiContext, value node: gui.NodeId, move value: gui.Selection` | `result<unit,gui.GuiError>` |
| `GuiContext.set_gap` | `mut self: gui.GuiContext, value node: gui.NodeId, value value: f64` | `result<unit,gui.GuiError>` |
| `GuiContext.set_block_background` | `mut self: gui.GuiContext, value node: gui.NodeId, value value: bool` | `result<unit,gui.GuiError>` |
| `GuiContext.set_enabled` | `mut self: gui.GuiContext, value node: gui.NodeId, value value: bool` | `result<unit,gui.GuiError>` |
| `GuiContext.set_text` | `mut self: gui.GuiContext, value node: gui.NodeId, ref text: str` | `result<u64,gui.GuiError>` |
| `GuiContext.text` | `ref self: gui.GuiContext, value node: gui.NodeId` | `result<str,gui.GuiError>` |
| `GuiContext.selection` | `ref self: gui.GuiContext, value node: gui.NodeId` | `result<gui.Selection,gui.GuiError>` |
| `GuiContext.composition` | `ref self: gui.GuiContext, value node: gui.NodeId` | `result<option<gui.CompositionInfo>,gui.GuiError>` |
| `GuiContext.node_info` | `ref self: gui.GuiContext, value node: gui.NodeId` | `result<gui.NodeInfo,gui.GuiError>` |
| `GuiContext.request_focus` | `mut self: gui.GuiContext, move node: option<gui.NodeId>` | `result<gui.FocusChange,gui.GuiError>` |
| `GuiContext.layout` | `mut self: gui.GuiContext` | `result<gui.LayoutInfo,gui.GuiError>` |
| `GuiContext.update` | `mut self: gui.GuiContext, move batch: window.EventBatch, value now: std.time.Instant` | `result<gui.GuiUpdate,gui.UpdateFailure>` |
| `GuiContext.draw` | `mut self: gui.GuiContext, mut frame: draw2d.Frame` | `result<unit,gui.GuiError>` |
| `GuiContext.close` | `move self: gui.GuiContext` | `result<unit,gui.GuiError>` |

| 型 | 定義 |
|---|---|
| `gui.GuiContext` | `resource kind 12; !Copy/!Send` |
| `gui.NodeId` | `opaque { context_slot: u64, context_generation: u64, node_slot: u32, node_generation: u32 } Copy=True, Send=True` |
| `gui.GuiErrorKind` | `InvalidNode() [tag 0]; WrongWidget() [tag 1]; InvalidArgument() [tag 2]; InvalidText() [tag 3]; LimitExceeded() [tag 4]; AlreadyAttached() [tag 5]; Closed() [tag 6]; Busy() [tag 7]; Unsupported() [tag 8]; NativeError() [tag 9]` |
| `gui.GuiError` | `record { kind: gui.GuiErrorKind, operation: str, os_code: option<i64> }` |
| `gui.WidgetKind` | `Panel() [tag 0]; Row() [tag 1]; Column() [tag 2]; Label() [tag 3]; Button() [tag 4]; TextInput() [tag 5]` |
| `gui.Align` | `Start() [tag 0]; Center() [tag 1]; End() [tag 2]; Stretch() [tag 3]` |
| `gui.Justify` | `Start() [tag 0]; Center() [tag 1]; End() [tag 2]; SpaceBetween() [tag 3]` |
| `gui.Visibility` | `Visible() [tag 0]; Hidden() [tag 1]; Collapsed() [tag 2]` |
| `gui.FocusChange` | `Applied() [tag 0]; Pending() [tag 1]` |
| `gui.Length` | `Auto() [tag 0]; Px(value: f64) [tag 1]; Fill(weight: f64) [tag 2]` |
| `gui.Insets` | `record { left: f64, top: f64, right: f64, bottom: f64 }; defaults={"left":0.0,"top":0.0,"right":0.0,"bottom":0.0}` |
| `gui.Layout` | `record { width: gui.Length, height: gui.Length, min_width: f64, min_height: f64, max_width: option<f64>, max_height: option<f64>, margin: gui.Insets, padding: option<gui.Insets>, align_x: gui.Align, align_y: gui.Align, justify: gui.Justify }; defaults={"width":{"variant":"Auto","fields":{}},"height":{"variant":"Auto","fields":{}},"min_width":0.0,"min_height":0.0,"max_width":null,"max_height":null,"margin":{"left":0.0,"top":0.0,"right":0.0,"bottom":0.0},"padding":null,"align_x":{"variant":"Stretch","fields":{}},"align_y":{"variant":"Stretch","fields":{}},"justify":{"variant":"Start","fields":{}}}` |
| `gui.SystemColor` | `WindowBackground() [tag 0]; WindowText() [tag 1]; ButtonFace() [tag 2]; ButtonText() [tag 3]; GrayText() [tag 4]; Highlight() [tag 5]; HighlightText() [tag 6]; Border() [tag 7]` |
| `gui.ColorSpec` | `System(role: gui.SystemColor) [tag 0]; Rgba(color: draw2d.Color) [tag 1]` |
| `gui.Palette` | `record { background: gui.ColorSpec, foreground: gui.ColorSpec, border: gui.ColorSpec }` |
| `gui.Theme` | `System() [tag 0]; Custom(palette: gui.Palette) [tag 1]` |
| `gui.Style` | `record { background: gui.ColorSpec, foreground: gui.ColorSpec, border: gui.ColorSpec, border_width: f64 }` |
| `gui.Semantics` | `record { name: option<str>, help: option<str>, automation_id: option<str>, labelled_by: option<gui.NodeId> }; defaults={"name":null,"help":null,"automation_id":null,"labelled_by":null}` |
| `gui.Selection` | `record { anchor_utf8: u64, active_utf8: u64 }` |
| `gui.TextInputOptions` | `record { read_only: bool, max_bytes: u64, placeholder: str }; defaults={"read_only":false,"max_bytes":1048576,"placeholder":""}` |
| `gui.WidgetSpec` | `Panel(block_background: bool) [tag 0]; Row(gap: f64, block_background: bool) [tag 1]; Column(gap: f64, block_background: bool) [tag 2]; Label(text: str, style: option<draw2d.TextStyle>) [tag 3]; Button(text: str, style: option<draw2d.TextStyle>) [tag 4]; TextInput(initial_text: str, style: option<draw2d.TextStyle>, options: gui.TextInputOptions) [tag 5]` |
| `gui.GuiLimits` | `record { nodes: u32, depth: u32, text_bytes: u64, total_text_bytes: u64, events: u32, uia_actions: u32, owned_bytes: u64 }; defaults={"nodes":4096,"depth":128,"text_bytes":1048576,"total_text_bytes":8388608,"events":4096,"uia_actions":128,"owned_bytes":67108864}` |
| `gui.GuiOptions` | `record { theme: gui.Theme, limits: gui.GuiLimits }; defaults={"theme":{"variant":"System","fields":{}},"limits":{"nodes":4096,"depth":128,"text_bytes":1048576,"total_text_bytes":8388608,"events":4096,"uia_actions":128,"owned_bytes":67108864}}` |
| `gui.CompositionInfo` | `record { preview: str, replacement: gui.Selection, cursor_utf8: u64 }` |
| `gui.NodeInfo` | `record { kind: gui.WidgetKind, enabled: bool, effective_enabled: bool, visibility: gui.Visibility, bounds: option<draw2d.Rect>, tree_revision: u64, layout_revision: u64 }` |
| `gui.LayoutInfo` | `record { tree_revision: u64, layout_revision: u64, needs_redraw: bool }` |
| `gui.InputSource` | `Pointer() [tag 0]; Keyboard() [tag 1]; Automation() [tag 2]; Programmatic() [tag 3]; Ime() [tag 4]` |
| `gui.Event` | `Clicked(node: gui.NodeId, source: gui.InputSource) [tag 0]; TextChanged(node: gui.NodeId, revision: u64, source: gui.InputSource) [tag 1]; Submitted(node: gui.NodeId, source: gui.InputSource) [tag 2]; FocusChanged(previous: option<gui.NodeId>, current: option<gui.NodeId>) [tag 3]; CompositionChanged(node: gui.NodeId, state: option<gui.CompositionInfo>) [tag 4]; EditRejected(node: gui.NodeId, error: gui.GuiError) [tag 5]; ActionFailed(node: option<gui.NodeId>, error: gui.GuiError) [tag 6]; InputReset(reason: window.ResetReason) [tag 7]; StateResyncRequired(tree_revision: u64) [tag 8]` |
| `gui.CaptureState` | `record { keyboard: bool, pointer: bool }` |
| `gui.GuiUpdate` | `record { events: list<gui.Event>, remaining_input: window.GameInputBatch, capture_state: gui.CaptureState, needs_redraw: bool, next_deadline: option<std.time.Deadline> }` |
| `gui.UpdateFailure` | `record { error: gui.GuiError, batch: window.EventBatch }` |
