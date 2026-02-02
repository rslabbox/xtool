use anyhow::{Context, Result};
use qiniu_upload_token::{credential::Credential as TokenCredential, prelude::*, UploadPolicy};
use std::time::Duration;

#[derive(Clone)]
pub struct QiniuClient {
    pub access_key: String,
    pub secret_key: String,
    pub domain: String,
    pub scheme: String,
    pub bucket_name: String,
    pub callback_url: String,
}

impl QiniuClient {
    pub fn new(
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
        domain: impl Into<String>,
        scheme: impl Into<String>,
        bucket_name: impl Into<String>,
        callback_url: impl Into<String>,
    ) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            domain: domain.into(),
            scheme: scheme.into(),
            bucket_name: bucket_name.into(),
            callback_url: callback_url.into(),
        }
    }

    pub fn generate_upload_token(&self, save_as_name: &str, lifetime: Duration) -> Result<String> {
        let callback_body = "key=$(key)&fname=$(fname)&fsize=$(fsize)&etag=$(etag)";
        let upload_policy = UploadPolicy::new_for_bucket(&self.bucket_name, lifetime)
            .insert_only()
            .object_lifetime(Duration::from_secs(24 * 60 * 60))
            .save_as(save_as_name, true)
            .callback([
                self.callback_url.as_str(),
            ], "", callback_body, "application/x-www-form-urlencoded")
            .build();

        let credential = TokenCredential::new(&self.access_key, &self.secret_key);
        let provider = upload_policy.into_static_upload_token_provider(credential, Default::default());
        let token = provider
            .to_token_string(Default::default())
            .context("Failed to generate upload token")?;
        Ok(token.into_owned())
    }

    pub fn get_download_url(&self, object_name: &str) -> String {
        format!("{}://{}/{}", self.scheme, self.domain, object_name)
    }
}
