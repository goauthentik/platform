#![cfg(unix)]

//! Builds the base VM image. CI runs this as its own step; elsewhere the
//! first `TestMachine::new()` builds it.

use ak_platform_e2e::{test_init, test_machine::base_image};

#[tokio::test(flavor = "multi_thread")]
async fn test_base_image() {
    test_init();
    base_image().await.expect("base image");
}
