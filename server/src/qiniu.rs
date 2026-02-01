use anyhow::{Context, Result};
use qiniu_upload_token::{
    credential::Credential as TokenCredential, prelude::*, UploadPolicy,
};
use qiniu_sdk::objects::{ObjectsManager, AfterDays};
use std::time::Duration;

#[derive(Clone)]
pub struct QiniuClient {
    pub access_key: String,
    pub secret_key: String,
    pub domain: String,
    pub scheme: String,
    pub bucket_name: String,
    // upload_url removed as SDK handles it
}

impl QiniuClient {
    pub fn new(
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
        domain: impl Into<String>,
        scheme: impl Into<String>,
        bucket_name: impl Into<String>,
    ) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            domain: domain.into(),
            scheme: scheme.into(),
            bucket_name: bucket_name.into(),
        }
    }

    pub fn generate_upload_token(
        &self,
        object_name: &str,
        lifetime: Duration,
    ) -> Result<String> {
        // Just generate the token for upload, expiration is set later upon completion or via policy
        // If we set it here via policy, it applies immediately after upload, which is good!
        // But the user asked for explicit "Server sets expiration time to 1 day" *after* upload notification.
        // However, setting it in policy is more robust.
        // But let's follow the user's explicit flow request: "Service sets file expiration... after client notifies".
        // So we remove the policy pre-set if it conflicts, or just set it again. Use standard policy.
        let upload_policy = UploadPolicy::new_for_object(&self.bucket_name, object_name, lifetime)
            .build();
        
        let credential = TokenCredential::new(&self.access_key, &self.secret_key);
        let provider = upload_policy.into_static_upload_token_provider(credential, Default::default());
        let token = provider
            .to_token_string(Default::default())
            .context("Failed to generate upload token")?;
        Ok(token.into_owned())
    }

    pub fn set_object_lifecycle(&self, object_name: &str, days: i64) -> Result<()> {
        let credential = qiniu_sdk::objects::apis::credential::Credential::new(&self.access_key, &self.secret_key);
        let objects_manager = ObjectsManager::builder(credential).build();
        
        objects_manager.bucket(&self.bucket_name)
            .modify_object_life_cycle(object_name)
            .delete_after_days(AfterDays::new(days as isize))
            .call()
            .context("Failed to set object lifecycle")?;
        Ok(())
    }

    pub fn get_download_url(&self, object_name: &str) -> String {
        format!("{}://{}/{}", self.scheme, self.domain, object_name)
    }
}
