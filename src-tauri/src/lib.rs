mod commands;
mod database_tools;
mod duckdb_tools;
mod mailpit_tools;
mod meilisearch_tools;
mod mongodb_tools;
mod nats_tools;
mod port_tools;
mod redis_tools;
mod sqlite_tools;
mod storage_tools;
mod tools;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::service_list,
            commands::service_install,
            commands::service_start,
            commands::service_stop,
            commands::service_restart,
            commands::service_metrics,
            commands::service_disk_usage,
            commands::environment_metrics,
            commands::environment_disk_usage,
            commands::service_config_read,
            commands::service_config_save,
            commands::service_logs,
            commands::redis_versions,
            commands::redis_version_select,
            commands::mysql_versions,
            commands::mysql_version_select,
            commands::postgres_versions,
            commands::postgres_version_select,
            storage_tools::service_cache_clean,
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
            port_tools::port_listeners,
            duckdb_tools::duckdb_status,
            duckdb_tools::duckdb_install,
            duckdb_tools::duckdb_query,
            sqlite_tools::sqlite_create,
            sqlite_tools::sqlite_overview,
            sqlite_tools::sqlite_tables,
            sqlite_tools::sqlite_execute,
            tools::data_format::data_format_transform,
            tools::json_diff::data_json_diff,
            tools::json_path::data_jsonpath_query,
            tools::jwt::jwt_decode,
            tools::jwt::jwt_verify_hmac,
            tools::jwt::jwt_sign_hmac,
            tools::jwk::jwk_inspect,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Zhiyu");
}
