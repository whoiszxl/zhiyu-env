mod clipboard;
mod commands;
mod database_tools;
mod diagnostics;
mod duckdb_tools;
mod http_tools;
mod kafka_tools;
mod mailpit_tools;
mod meilisearch_tools;
mod mock_tools;
mod mongodb_tools;
mod nats_tools;
mod port_tools;
mod qr_tools;
mod redis_tools;
mod s3_tools;
mod settings;
mod sqlite_tools;
mod ssh_tools;
mod storage_tools;
mod tools;
mod tray;
mod uninstall;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    if let Some(win) = app.get_webview_window("clipboard") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = win.hide();
                        } else {
                            let _ = win.show();
                            let _ = win.set_focus();
                            let _ = win.center();
                        }
                    }
                })
                .build(),
        )
        .manage(clipboard::commands::ClipboardState(std::sync::Mutex::new(
            None,
        )))
        .manage(ssh_tools::SshTerminalState::default())
        .setup(|app| {
            let settings = settings::load_settings();
            let _ = settings::apply_log_retention(&settings);
            tray::setup(app)?;

            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
                let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    eprintln!("全局快捷键注册失败: {e}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::service_list,
            commands::service_install,
            commands::service_install_cancel,
            commands::service_start,
            commands::service_stop,
            commands::service_restart,
            commands::service_force_stop,
            commands::service_repair,
            uninstall::service_version_uninstall,
            commands::service_stop_all,
            commands::service_metrics,
            commands::service_disk_usage,
            commands::environment_metrics,
            commands::environment_disk_usage,
            diagnostics::app_diagnostics_run,
            diagnostics::app_diagnostics_repair,
            commands::open_url,
            settings::app_settings_get,
            settings::app_settings_save,
            settings::app_background_import,
            settings::app_background_remove,
            settings::app_update_check,
            commands::service_config_read,
            commands::service_config_save,
            commands::service_logs,
            commands::redis_versions,
            commands::redis_version_select,
            commands::mysql_versions,
            commands::mysql_version_select,
            commands::postgres_versions,
            commands::postgres_version_select,
            commands::nginx_versions,
            commands::nginx_version_select,
            commands::service_versions,
            commands::service_version_select,
            commands::open_url,
            commands::open_path,
            commands::nginx_html_list,
            commands::nginx_html_read,
            commands::nginx_html_write,
            commands::nginx_html_delete,
            commands::caddy_html_list,
            commands::caddy_html_read,
            commands::caddy_html_write,
            commands::caddy_html_delete,
            storage_tools::service_cache_clean,
            storage_tools::app_cache_clean_all,
            storage_tools::service_backup_list,
            storage_tools::service_backup_create,
            storage_tools::service_backup_restore,
            redis_tools::redis_overview,
            redis_tools::redis_scan_keys,
            redis_tools::redis_key_detail,
            redis_tools::redis_execute,
            database_tools::database_overview,
            database_tools::database_list,
            database_tools::database_tables,
            database_tools::database_table_detail,
            database_tools::database_execute,
            mongodb_tools::mongo_overview,
            mongodb_tools::mongo_databases,
            mongodb_tools::mongo_collections,
            mongodb_tools::mongo_collection_detail,
            mongodb_tools::mongo_execute,
            mailpit_tools::mailpit_overview,
            mailpit_tools::mailpit_messages,
            mailpit_tools::mailpit_message_detail,
            meilisearch_tools::meilisearch_overview,
            meilisearch_tools::meilisearch_indexes,
            meilisearch_tools::meilisearch_add_documents,
            meilisearch_tools::meilisearch_search,
            nats_tools::nats_overview,
            nats_tools::nats_publish,
            nats_tools::nats_receive,
            kafka_tools::kafka_overview,
            kafka_tools::kafka_topics,
            kafka_tools::kafka_topic_create,
            kafka_tools::kafka_topic_delete,
            kafka_tools::kafka_publish,
            port_tools::port_listeners,
            duckdb_tools::duckdb_status,
            duckdb_tools::duckdb_install,
            duckdb_tools::duckdb_query,
            sqlite_tools::sqlite_create,
            sqlite_tools::sqlite_overview,
            sqlite_tools::sqlite_tables,
            sqlite_tools::sqlite_execute,
            tools::data_format::data_format_transform,
            tools::data_format::data_csv_transform,
            tools::json_diff::data_json_diff,
            tools::json_path::data_jsonpath_query,
            tools::jwt::jwt_decode,
            tools::jwt::jwt_verify_hmac,
            tools::jwt::jwt_sign_hmac,
            tools::jwk::jwk_inspect,
            commands::service_test_connection,
            clipboard::commands::clipboard_start,
            clipboard::commands::clipboard_stop,
            clipboard::commands::clipboard_pause,
            clipboard::commands::clipboard_resume,
            clipboard::commands::clipboard_status,
            clipboard::commands::clipboard_list,
            clipboard::commands::clipboard_copy,
            clipboard::commands::clipboard_pin,
            clipboard::commands::clipboard_delete,
            clipboard::commands::clipboard_clear,
            clipboard::commands::clipboard_settings_get,
            clipboard::commands::clipboard_settings_save,
            s3_tools::s3_list_buckets,
            s3_tools::s3_list_objects,
            s3_tools::s3_get_object,
            s3_tools::s3_put_object,
            s3_tools::s3_put_file,
            s3_tools::s3_delete_object,
            s3_tools::s3_presigned_url,
            s3_tools::s3_config_get,
            s3_tools::s3_config_save,
            mock_tools::mock_api_state,
            mock_tools::mock_api_save_routes,
            mock_tools::mock_api_start,
            mock_tools::mock_api_stop,
            mock_tools::mock_api_clear_requests,
            http_tools::http_request_execute,
            qr_tools::qr_code_generate,
            ssh_tools::ssh_profiles_list,
            ssh_tools::ssh_profile_save,
            ssh_tools::ssh_profile_delete,
            ssh_tools::ssh_host_key_preview,
            ssh_tools::ssh_host_key_trust,
            ssh_tools::ssh_connection_test,
            ssh_tools::ssh_command_execute,
            ssh_tools::ssh_terminal_connect,
            ssh_tools::ssh_terminal_input,
            ssh_tools::ssh_terminal_resize,
            ssh_tools::ssh_terminal_disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Zhiyu");
}
