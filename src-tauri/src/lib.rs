mod ai_chat;
mod ai_runtime;
mod ai_settings;
mod ai_tools;
mod clipboard;
mod commands;
mod database_tools;
mod diagnostics;
mod duckdb_tools;
mod http_tools;
mod influxdb_tools;
mod kafka_tools;
mod local_domains;
mod mailpit_tools;
mod meilisearch_tools;
mod mock_tools;
mod mongodb_tools;
mod nats_tools;
mod network_tools;
mod olap_tools;
mod port_tools;
mod qr_tools;
mod redis_tools;
mod rss_ai;
mod rss_tools;
mod runtime_tools;
mod s3_tools;
mod scheduled_tasks;
mod settings;
mod sqlite_tools;
mod ssh_tools;
mod storage_tools;
mod test_data;
mod tools;
mod tray;
mod uninstall;
mod zeromq_tools;

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
        .manage(ai_chat::AiChatState::default())
        .manage(ai_tools::AiToolState::default())
        .manage(rss_ai::RssAiState::default())
        .manage(ssh_tools::SshTerminalState::default())
        .setup(|app| {
            let settings = settings::load_settings();
            let _ = settings::apply_log_retention(&settings);
            tray::setup(app)?;
            rss_tools::start_scheduler();
            scheduled_tasks::start_scheduler();

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
            ai_settings::ai_settings_get,
            ai_settings::ai_settings_save,
            ai_settings::ai_connection_test,
            ai_settings::ai_avatar_import,
            ai_settings::ai_avatar_remove,
            ai_chat::ai_chat_sessions_list,
            ai_chat::ai_chat_session_create,
            ai_chat::ai_chat_session_delete,
            ai_chat::ai_chat_messages_list,
            ai_chat::ai_chat_send,
            ai_chat::ai_chat_cancel,
            ai_tools::ai_tool_generate,
            ai_tools::ai_tool_cancel,
            rss_ai::rss_ai_results_list,
            rss_ai::rss_ai_generate,
            rss_ai::rss_ai_cancel,
            rss_ai::rss_ai_result_delete,
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
            influxdb_tools::influxdb_overview,
            influxdb_tools::influxdb_databases,
            influxdb_tools::influxdb_database_create,
            influxdb_tools::influxdb_database_delete,
            influxdb_tools::influxdb_query,
            influxdb_tools::influxdb_write,
            nats_tools::nats_overview,
            nats_tools::nats_publish,
            nats_tools::nats_receive,
            kafka_tools::kafka_overview,
            kafka_tools::kafka_topics,
            kafka_tools::kafka_topic_create,
            kafka_tools::kafka_topic_delete,
            kafka_tools::kafka_publish,
            port_tools::port_listeners,
            network_tools::network_diagnose,
            network_tools::network_proxy_settings,
            zeromq_tools::zeromq_publish,
            zeromq_tools::zeromq_subscribe,
            zeromq_tools::zeromq_push,
            zeromq_tools::zeromq_pull,
            olap_tools::olap_profile_list,
            olap_tools::olap_profile_save,
            olap_tools::olap_profile_delete,
            olap_tools::olap_connection_test,
            olap_tools::olap_database_list,
            olap_tools::olap_table_list,
            olap_tools::olap_execute,
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
            http_tools::http_workspace_get,
            http_tools::http_workspace_save,
            local_domains::local_domains_get,
            local_domains::local_domains_save,
            local_domains::local_domains_apply,
            local_domains::local_domains_restore,
            local_domains::local_domain_target_check,
            test_data::test_data_export,
            qr_tools::qr_code_generate,
            rss_tools::rss_feeds_list,
            rss_tools::rss_feed_add,
            rss_tools::rss_feed_delete,
            rss_tools::rss_feed_update,
            rss_tools::rss_feed_refresh,
            rss_tools::rss_refresh_due,
            rss_tools::rss_entries_list,
            rss_tools::rss_entry_read,
            rss_tools::rss_entry_star,
            rss_tools::rss_mark_all_read,
            rss_tools::rss_import_opml,
            rss_tools::rss_export_opml,
            scheduled_tasks::scheduled_tasks_list,
            scheduled_tasks::scheduled_task_save,
            scheduled_tasks::scheduled_task_delete,
            scheduled_tasks::scheduled_task_toggle,
            scheduled_tasks::scheduled_task_cancel,
            scheduled_tasks::scheduled_task_run,
            scheduled_tasks::scheduled_task_history,
            runtime_tools::runtime_overview,
            runtime_tools::runtime_install,
            runtime_tools::runtime_select,
            runtime_tools::runtime_uninstall,
            runtime_tools::runtime_diagnose,
            runtime_tools::runtime_go_proxy_set,
            runtime_tools::runtime_projects_list,
            runtime_tools::runtime_project_save,
            runtime_tools::runtime_project_delete,
            runtime_tools::runtime_project_manifest_export,
            runtime_tools::runtime_project_manifest_import,
            runtime_tools::runtime_open_terminal,
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
