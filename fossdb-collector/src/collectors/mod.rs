pub mod helpers;

#[cfg(feature = "collector-rust")]
pub mod crates_io;
#[cfg(feature = "collector-libraries-io")]
pub mod libraries_io;
#[cfg(feature = "collector-nixpkgs")]
pub mod nixpkgs;

#[async_trait::async_trait]
pub trait Collector: Send + Sync {
    fn name(&self) -> &str;
    async fn collect(&self, client: std::sync::Arc<crate::client::CollectorClient>) -> anyhow::Result<()>;
}
