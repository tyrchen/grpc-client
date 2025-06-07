use once_cell::sync::OnceCell;

static GRPC_SERVER: OnceCell<()> = OnceCell::new();

#[allow(dead_code)]
pub async fn start_grpc_server() {
    let port = 50051;
    GRPC_SERVER.get_or_init(|| {
        tokio::spawn(async move {
            if let Err(e) = sample_server::start_grpc_server(port).await {
                eprintln!("Error starting server: {}", e);
            }
        });
    });
}

/// Async test helper macros and utilities
#[macro_export]
macro_rules! test_async {
    ($test_name:ident, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            $test_body
        }
    };
}

/// Serial test helper for tests that can't run in parallel
#[macro_export]
macro_rules! test_serial {
    ($test_name:ident, $test_body:expr) => {
        #[tokio::test]
        #[serial_test::serial]
        async fn $test_name() {
            $test_body
        }
    };
}

/// Initialize test logging (call once per test suite)
static INIT: OnceCell<()> = OnceCell::new();

#[allow(dead_code)]
pub fn init_test_logging() {
    INIT.get_or_init(|| {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .init();
    });
}

/// Assert that a JSON response contains expected fields
#[macro_export]
macro_rules! assert_json_contains {
    ($response:expr, $($key:expr => $value:expr),+) => {
        let json: serde_json::Value = $response;
        $(
            assert_eq!(
                json.get($key),
                Some(&$value),
                "Expected key '{}' to have value '{:?}', but got '{:?}'",
                $key,
                $value,
                json.get($key)
            );
        )+
    };
}
