use std::{env, fs, path::PathBuf, str::FromStr};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            setup_sqlx().await;
        });

    setup_store_flag();

    println!("cargo:rerun-if-changed=protobuff/");
    prost_build::compile_protos(&["protobuff/sync_objects.proto"], &["protobuff/"]).unwrap();

    tauri_build::build()
}

fn setup_store_flag() {
    println!("cargo::rustc-check-cfg=cfg(store)");
    println!("cargo:rerun-if-env-changed=STORE_BUILD");
    if env::var("STORE_BUILD").is_ok() || cfg!(feature = "store") {
        println!("cargo:rustc-cfg=store");
    }
}

async fn setup_sqlx() {
    println!("cargo:rerun-if-changed=migrations/");

    let db_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory")).join("sqlx.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display()).replace('\\', "/");
    if db_path.exists() {
        fs::remove_file(&db_path).expect("Cannot remove old compile-time database");
    }
    println!("cargo:rustc-env=DATABASE_URL={db_url}");

    let options = SqliteConnectOptions::from_str(&db_url)
        .expect("Could not create sqlite connect options!")
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("Cannot connect to database");
    sqlx::migrate!("./migrations/")
        .run(&pool)
        .await
        .expect("Error applying the migrations!");
}
