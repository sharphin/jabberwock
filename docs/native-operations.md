# 個別操作のC ABI v1

状態: 実装前の設計。宣言と配置の正本は [生成ヘッダー](stdlib-v1/stdlib-abi-v1.h)。実装したC関数の一覧ではない。

すべての公開操作は固定IDと型付きの専用Args/Outを持つ。呼出し規則とcodec、所有、失敗は [C境界の契約](stdlib-v1-contracts.md) に従う。

| ID | 公開操作 | C symbol | Argsのbyte/整列 | Outのbyte | 所有slot数 |
|---|---|---|---:|---:|---:|
| 0x00010001 | std.fs.Path.from_text | jw_fs_path_from_text_v1 | 16/8 | 48 | 0 |
| 0x00010002 | std.fs.Path.to_text | jw_fs_path_to_text_v1 | 16/8 | 48 | 0 |
| 0x00010003 | std.fs.Path.display | jw_fs_path_display_v1 | 16/8 | 48 | 0 |
| 0x00010004 | std.fs.Path.file_name | jw_fs_path_file_name_v1 | 16/8 | 48 | 0 |
| 0x00010005 | std.fs.Path.join | jw_fs_path_join_v1 | 32/8 | 48 | 0 |
| 0x00010006 | std.fs.Path.absolute | jw_fs_path_absolute_v1 | 16/8 | 48 | 0 |
| 0x00010007 | std.fs.Path.canonical | jw_fs_path_canonical_v1 | 16/8 | 48 | 0 |
| 0x00010008 | std.fs.WindowsShare.defaults | jw_fs_windows_share_defaults_v1 | 8/8 | 48 | 0 |
| 0x00010009 | std.fs.OpenOptions.defaults | jw_fs_open_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x0001000a | std.fs.open | jw_fs_open_v1 | 32/8 | 72 | 1 |
| 0x0001000b | std.fs.read_bytes | jw_fs_read_bytes_v1 | 24/8 | 48 | 0 |
| 0x0001000c | std.fs.read_text | jw_fs_read_text_v1 | 24/8 | 48 | 0 |
| 0x0001000d | std.fs.read_dir | jw_fs_read_dir_v1 | 24/8 | 48 | 0 |
| 0x0001000e | std.fs.write_bytes | jw_fs_write_bytes_v1 | 48/8 | 48 | 0 |
| 0x0001000f | std.fs.metadata | jw_fs_metadata_v1 | 32/8 | 48 | 0 |
| 0x00010010 | std.fs.create_dir | jw_fs_create_dir_v1 | 16/8 | 48 | 0 |
| 0x00010011 | std.fs.remove_file | jw_fs_remove_file_v1 | 16/8 | 48 | 0 |
| 0x00010012 | std.fs.remove_empty_dir | jw_fs_remove_empty_dir_v1 | 16/8 | 48 | 0 |
| 0x00010013 | std.fs.rename | jw_fs_rename_v1 | 48/8 | 48 | 0 |
| 0x00010014 | std.fs.File.read | jw_fs_file_read_v1 | 32/8 | 48 | 0 |
| 0x00010015 | std.fs.File.write | jw_fs_file_write_v1 | 40/8 | 48 | 0 |
| 0x00010016 | std.fs.File.write_all | jw_fs_file_write_all_v1 | 40/8 | 48 | 0 |
| 0x00010017 | std.fs.File.seek | jw_fs_file_seek_v1 | 48/8 | 48 | 0 |
| 0x00010018 | std.fs.File.length | jw_fs_file_length_v1 | 24/8 | 48 | 0 |
| 0x00010019 | std.fs.File.metadata | jw_fs_file_metadata_v1 | 24/8 | 48 | 0 |
| 0x0001001a | std.fs.File.set_length | jw_fs_file_set_length_v1 | 32/8 | 48 | 0 |
| 0x0001001b | std.fs.File.flush | jw_fs_file_flush_v1 | 24/8 | 48 | 0 |
| 0x0001001c | std.fs.File.sync | jw_fs_file_sync_v1 | 24/8 | 48 | 0 |
| 0x0001001d | std.fs.File.close | jw_fs_file_close_v1 | 24/8 | 48 | 0 |
| 0x00020001 | std.time.now | jw_time_now_v1 | 8/8 | 48 | 0 |
| 0x00020002 | std.time.elapsed | jw_time_elapsed_v1 | 16/8 | 48 | 0 |
| 0x00020003 | std.time.Duration.from_millis | jw_time_duration_from_millis_v1 | 8/8 | 48 | 0 |
| 0x00020004 | std.time.Duration.from_seconds | jw_time_duration_from_seconds_v1 | 8/8 | 48 | 0 |
| 0x00020005 | std.time.Duration.from_nanos | jw_time_duration_from_nanos_v1 | 8/8 | 48 | 0 |
| 0x00020006 | std.time.Duration.nanos | jw_time_duration_nanos_v1 | 16/8 | 48 | 0 |
| 0x00020007 | std.time.Deadline.infinite | jw_time_deadline_infinite_v1 | 8/8 | 48 | 0 |
| 0x00020008 | std.time.deadline_after | jw_time_deadline_after_v1 | 16/8 | 48 | 0 |
| 0x00020009 | std.time.deadline_at | jw_time_deadline_at_v1 | 16/8 | 48 | 0 |
| 0x00030001 | std.net.IpAddress.v4 | jw_net_ip_address_v4_v1 | 16/8 | 48 | 0 |
| 0x00030002 | std.net.IpAddress.v6 | jw_net_ip_address_v6_v1 | 16/8 | 48 | 0 |
| 0x00030003 | std.net.SocketAddress.create | jw_net_socket_address_create_v1 | 24/8 | 48 | 0 |
| 0x00030004 | std.net.SocketAddress.ip | jw_net_socket_address_ip_v1 | 16/8 | 48 | 0 |
| 0x00030005 | std.net.SocketAddress.port | jw_net_socket_address_port_v1 | 16/8 | 48 | 0 |
| 0x00030006 | std.net.SocketAddress.scope | jw_net_socket_address_scope_v1 | 16/8 | 48 | 0 |
| 0x00030007 | std.net.BindOptions.defaults | jw_net_bind_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00030008 | std.net.resolve | jw_net_resolve_v1 | 40/8 | 48 | 0 |
| 0x00030009 | std.net.connect | jw_net_connect_v1 | 32/8 | 72 | 1 |
| 0x0003000a | std.net.bind | jw_net_bind_v1 | 32/8 | 72 | 1 |
| 0x0003000b | std.net.TcpListener.accept | jw_net_tcp_listener_accept_v1 | 40/8 | 72 | 1 |
| 0x0003000c | std.net.TcpListener.address | jw_net_tcp_listener_address_v1 | 24/8 | 48 | 0 |
| 0x0003000d | std.net.TcpListener.close | jw_net_tcp_listener_close_v1 | 24/8 | 48 | 0 |
| 0x0003000e | std.net.TcpStream.read | jw_net_tcp_stream_read_v1 | 48/8 | 48 | 0 |
| 0x0003000f | std.net.TcpStream.read_exact | jw_net_tcp_stream_read_exact_v1 | 48/8 | 48 | 0 |
| 0x00030010 | std.net.TcpStream.write | jw_net_tcp_stream_write_v1 | 56/8 | 48 | 0 |
| 0x00030011 | std.net.TcpStream.write_all | jw_net_tcp_stream_write_all_v1 | 56/8 | 48 | 0 |
| 0x00030012 | std.net.TcpStream.local_address | jw_net_tcp_stream_local_address_v1 | 24/8 | 48 | 0 |
| 0x00030013 | std.net.TcpStream.peer_address | jw_net_tcp_stream_peer_address_v1 | 24/8 | 48 | 0 |
| 0x00030014 | std.net.TcpStream.shutdown_write | jw_net_tcp_stream_shutdown_write_v1 | 24/8 | 48 | 0 |
| 0x00030015 | std.net.TcpStream.close | jw_net_tcp_stream_close_v1 | 24/8 | 48 | 0 |
| 0x00040001 | std.tls.Options.defaults | jw_tls_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00040002 | std.tls.connect | jw_tls_connect_v1 | 64/8 | 72 | 1 |
| 0x00040003 | std.tls.start_tls | jw_tls_start_tls_v1 | 72/8 | 72 | 1 |
| 0x00040004 | std.tls.TlsStream.read | jw_tls_tls_stream_read_v1 | 48/8 | 48 | 0 |
| 0x00040005 | std.tls.TlsStream.write | jw_tls_tls_stream_write_v1 | 56/8 | 48 | 0 |
| 0x00040006 | std.tls.TlsStream.write_all | jw_tls_tls_stream_write_all_v1 | 56/8 | 48 | 0 |
| 0x00040007 | std.tls.TlsStream.close | jw_tls_tls_stream_close_v1 | 40/8 | 48 | 0 |
| 0x00050001 | postgres.connect | jw_postgres_connect_v1 | 16/8 | 72 | 1 |
| 0x00050002 | postgres.QueryOptions.defaults | jw_postgres_query_options_defaults_v1 | 16/8 | 48 | 0 |
| 0x00050003 | postgres.TxOptions.defaults | jw_postgres_tx_options_defaults_v1 | 16/8 | 48 | 0 |
| 0x00050004 | postgres.Connection.run | jw_postgres_connection_run_v1 | 72/8 | 48 | 0 |
| 0x00050005 | postgres.Transaction.run | jw_postgres_transaction_run_v1 | 72/8 | 48 | 0 |
| 0x00050006 | postgres.Connection.begin | jw_postgres_connection_begin_v1 | 40/8 | 72 | 1 |
| 0x00050007 | postgres.Connection.close | jw_postgres_connection_close_v1 | 24/8 | 48 | 0 |
| 0x00050008 | postgres.Transaction.commit | jw_postgres_transaction_commit_v1 | 40/8 | 72 | 1 |
| 0x00050009 | postgres.Transaction.rollback | jw_postgres_transaction_rollback_v1 | 40/8 | 72 | 1 |
| 0x0005000a | postgres.QueryResult.column_index | jw_postgres_query_result_column_index_v1 | 32/8 | 48 | 0 |
| 0x0005000b | postgres.Row.cell | jw_postgres_row_cell_v1 | 24/8 | 48 | 0 |
| 0x0005000c | postgres.Cell.as_bool | jw_postgres_cell_as_bool_v1 | 16/8 | 48 | 0 |
| 0x0005000d | postgres.Cell.as_i64 | jw_postgres_cell_as_i64_v1 | 16/8 | 48 | 0 |
| 0x0005000e | postgres.Cell.as_f64 | jw_postgres_cell_as_f64_v1 | 16/8 | 48 | 0 |
| 0x0005000f | postgres.Cell.as_text | jw_postgres_cell_as_text_v1 | 16/8 | 48 | 0 |
| 0x00050010 | postgres.Cell.as_bytes | jw_postgres_cell_as_bytes_v1 | 16/8 | 48 | 0 |
| 0x00060001 | window.Options.defaults | jw_window_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00060002 | window.create_window | jw_window_create_window_v1 | 16/8 | 72 | 1 |
| 0x00060003 | window.Window.poll_events | jw_window_window_poll_events_v1 | 24/8 | 48 | 0 |
| 0x00060004 | window.Window.wait_events | jw_window_window_wait_events_v1 | 40/8 | 48 | 0 |
| 0x00060005 | window.Window.info | jw_window_window_info_v1 | 24/8 | 48 | 0 |
| 0x00060006 | window.Window.request_redraw | jw_window_window_request_redraw_v1 | 24/8 | 48 | 0 |
| 0x00060007 | window.Window.close | jw_window_window_close_v1 | 24/8 | 48 | 0 |
| 0x00060008 | window.EventBatch.events | jw_window_event_batch_events_v1 | 16/8 | 48 | 0 |
| 0x00060009 | window.GameInputBatch.events | jw_window_game_input_batch_events_v1 | 16/8 | 48 | 0 |
| 0x00070001 | draw2d.RendererOptions.defaults | jw_draw2d_renderer_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00070002 | draw2d.TextureOptions.defaults | jw_draw2d_texture_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00070003 | draw2d.TextStyle.defaults | jw_draw2d_text_style_defaults_v1 | 8/8 | 48 | 0 |
| 0x00070004 | draw2d.PresentOptions.defaults | jw_draw2d_present_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00070005 | draw2d.create_renderer | jw_draw2d_create_renderer_v1 | 40/8 | 72 | 1 |
| 0x00070006 | draw2d.create_text_layout | jw_draw2d_create_text_layout_v1 | 32/8 | 72 | 1 |
| 0x00070007 | draw2d.Texture.from_rgba8 | jw_draw2d_texture_from_rgba8_v1 | 72/8 | 72 | 1 |
| 0x00070008 | draw2d.Renderer.info | jw_draw2d_renderer_info_v1 | 24/8 | 48 | 0 |
| 0x00070009 | draw2d.Renderer.begin_frame | jw_draw2d_renderer_begin_frame_v1 | 40/8 | 72 | 1 |
| 0x0007000a | draw2d.Renderer.close | jw_draw2d_renderer_close_v1 | 24/8 | 48 | 0 |
| 0x0007000b | draw2d.Texture.close | jw_draw2d_texture_close_v1 | 24/8 | 48 | 0 |
| 0x0007000c | draw2d.TextLayout.close | jw_draw2d_text_layout_close_v1 | 24/8 | 48 | 0 |
| 0x0007000d | draw2d.TextLayout.info | jw_draw2d_text_layout_info_v1 | 24/8 | 48 | 0 |
| 0x0007000e | draw2d.Frame.fill_rect | jw_draw2d_frame_fill_rect_v1 | 56/8 | 48 | 0 |
| 0x0007000f | draw2d.Frame.stroke_rect | jw_draw2d_frame_stroke_rect_v1 | 64/8 | 48 | 0 |
| 0x00070010 | draw2d.Frame.triangle | jw_draw2d_frame_triangle_v1 | 88/8 | 48 | 0 |
| 0x00070011 | draw2d.Frame.image | jw_draw2d_frame_image_v1 | 96/8 | 48 | 0 |
| 0x00070012 | draw2d.Frame.text | jw_draw2d_frame_text_v1 | 80/8 | 48 | 0 |
| 0x00070013 | draw2d.Frame.push_clip | jw_draw2d_frame_push_clip_v1 | 40/8 | 48 | 0 |
| 0x00070014 | draw2d.Frame.pop_clip | jw_draw2d_frame_pop_clip_v1 | 24/8 | 48 | 0 |
| 0x00070015 | draw2d.Frame.push_transform | jw_draw2d_frame_push_transform_v1 | 40/8 | 48 | 0 |
| 0x00070016 | draw2d.Frame.pop_transform | jw_draw2d_frame_pop_transform_v1 | 24/8 | 48 | 0 |
| 0x00070017 | draw2d.Frame.finish | jw_draw2d_frame_finish_v1 | 40/8 | 48 | 0 |
| 0x00070018 | draw2d.Frame.abort | jw_draw2d_frame_abort_v1 | 24/8 | 48 | 0 |
| 0x00080001 | image.Limits.defaults | jw_image_limits_defaults_v1 | 8/8 | 48 | 0 |
| 0x00080002 | image.decode | jw_image_decode_v1 | 32/8 | 48 | 0 |
| 0x00080003 | image.load | jw_image_load_v1 | 32/8 | 48 | 0 |
| 0x00090001 | gui.Insets.defaults | jw_gui_insets_defaults_v1 | 8/8 | 48 | 0 |
| 0x00090002 | gui.Layout.defaults | jw_gui_layout_defaults_v1 | 8/8 | 48 | 0 |
| 0x00090003 | gui.Semantics.defaults | jw_gui_semantics_defaults_v1 | 8/8 | 48 | 0 |
| 0x00090004 | gui.TextInputOptions.defaults | jw_gui_text_input_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00090005 | gui.GuiLimits.defaults | jw_gui_gui_limits_defaults_v1 | 8/8 | 48 | 0 |
| 0x00090006 | gui.GuiOptions.defaults | jw_gui_gui_options_defaults_v1 | 8/8 | 48 | 0 |
| 0x00090007 | gui.create_gui | jw_gui_create_gui_v1 | 40/8 | 72 | 1 |
| 0x00090008 | gui.GuiContext.root | jw_gui_gui_context_root_v1 | 24/8 | 48 | 0 |
| 0x00090009 | gui.GuiContext.create | jw_gui_gui_context_create_v1 | 88/8 | 48 | 0 |
| 0x0009000a | gui.GuiContext.reparent | jw_gui_gui_context_reparent_v1 | 64/8 | 48 | 0 |
| 0x0009000b | gui.GuiContext.remove | jw_gui_gui_context_remove_v1 | 40/8 | 48 | 0 |
| 0x0009000c | gui.GuiContext.set_layout | jw_gui_gui_context_set_layout_v1 | 56/8 | 48 | 0 |
| 0x0009000d | gui.GuiContext.set_style | jw_gui_gui_context_set_style_v1 | 56/8 | 48 | 0 |
| 0x0009000e | gui.GuiContext.set_text_style | jw_gui_gui_context_set_text_style_v1 | 56/8 | 48 | 0 |
| 0x0009000f | gui.GuiContext.set_input_options | jw_gui_gui_context_set_input_options_v1 | 56/8 | 48 | 0 |
| 0x00090010 | gui.GuiContext.set_semantics | jw_gui_gui_context_set_semantics_v1 | 56/8 | 48 | 0 |
| 0x00090011 | gui.GuiContext.set_visibility | jw_gui_gui_context_set_visibility_v1 | 56/8 | 48 | 0 |
| 0x00090012 | gui.GuiContext.set_selection | jw_gui_gui_context_set_selection_v1 | 56/8 | 48 | 0 |
| 0x00090013 | gui.GuiContext.set_gap | jw_gui_gui_context_set_gap_v1 | 48/8 | 48 | 0 |
| 0x00090014 | gui.GuiContext.set_block_background | jw_gui_gui_context_set_block_background_v1 | 48/8 | 48 | 0 |
| 0x00090015 | gui.GuiContext.set_enabled | jw_gui_gui_context_set_enabled_v1 | 48/8 | 48 | 0 |
| 0x00090016 | gui.GuiContext.set_text | jw_gui_gui_context_set_text_v1 | 56/8 | 48 | 0 |
| 0x00090017 | gui.GuiContext.text | jw_gui_gui_context_text_v1 | 40/8 | 48 | 0 |
| 0x00090018 | gui.GuiContext.selection | jw_gui_gui_context_selection_v1 | 40/8 | 48 | 0 |
| 0x00090019 | gui.GuiContext.composition | jw_gui_gui_context_composition_v1 | 40/8 | 48 | 0 |
| 0x0009001a | gui.GuiContext.node_info | jw_gui_gui_context_node_info_v1 | 40/8 | 48 | 0 |
| 0x0009001b | gui.GuiContext.request_focus | jw_gui_gui_context_request_focus_v1 | 40/8 | 48 | 0 |
| 0x0009001c | gui.GuiContext.layout | jw_gui_gui_context_layout_v1 | 24/8 | 48 | 0 |
| 0x0009001d | gui.GuiContext.update | jw_gui_gui_context_update_v1 | 56/8 | 48 | 0 |
| 0x0009001e | gui.GuiContext.draw | jw_gui_gui_context_draw_v1 | 48/8 | 48 | 0 |
| 0x0009001f | gui.GuiContext.close | jw_gui_gui_context_close_v1 | 24/8 | 48 | 0 |

## 不透明な値のcodec ID

| ID | 型 | Copy/Send |
|---:|---|---|
| 1 | std.fs.Path | True/True |
| 2 | std.time.Duration | True/True |
| 3 | std.time.Instant | True/True |
| 4 | std.time.Deadline | True/True |
| 5 | std.net.IpAddress | True/True |
| 6 | std.net.SocketAddress | True/True |
| 7 | window.WindowId | True/True |
| 8 | window.EventBatch | False/True |
| 9 | window.GameInputBatch | False/True |
| 10 | gui.NodeId | True/True |
