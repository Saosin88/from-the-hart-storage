use aws_config::SdkConfig;
use tokio::sync::OnceCell;

static AWS_SDK_CONFIG: OnceCell<SdkConfig> = OnceCell::const_new();

/// Returns a shared AWS SDK config, loaded once at first call.
pub(crate) async fn get_aws_config() -> &'static SdkConfig {
    AWS_SDK_CONFIG
        .get_or_init(|| async {
            aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
        })
        .await
}
